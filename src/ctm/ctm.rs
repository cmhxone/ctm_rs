use std::{
    collections::HashMap,
    error::Error,
    thread,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use tokio::{
    sync::{broadcast, mpsc},
    time::timeout,
};

use crate::{
    cisco::{
        control::query_agent_state_conf::QueryAgentStateConf, session::OpenConf,
        supervisor::agent_team_config_event::AgentTeamConfigEvent, Deserializable, MessageType,
    },
    ctm::cti_client::CTIClient,
    event::{broker_event::BrokerEvent, cti_event::CTIEvent},
};

use super::{
    acceptor::{tcp_acceptor::TCPAcceptor, Acceptor},
    agent_info::AgentInfo,
};

pub struct CTM {
    is_active: bool,
    cti_client: CTIClient,
    cti_event_channel_rx: mpsc::Receiver<CTIEvent>,
    cti_event_channel_tx: mpsc::Sender<CTIEvent>,
    broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
    broker_event_channel_tx: broadcast::Sender<BrokerEvent>,
    agent_info_map: HashMap<String, AgentInfo>,
}

impl CTM {
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let is_active = true;
        let (cti_event_channel_tx, cti_event_channel_rx) = mpsc::channel::<CTIEvent>(1_024);
        let (broker_event_channel_tx, broker_event_channel_rx) =
            broadcast::channel::<BrokerEvent>(1_024);

        let cti_client = CTIClient::new(
            is_active,
            cti_event_channel_tx.clone(),
            broker_event_channel_rx.resubscribe(),
        )
        .await?;

        let agent_info_map = HashMap::new();

        Ok(Self {
            is_active,
            cti_client,
            cti_event_channel_rx,
            cti_event_channel_tx,
            broker_event_channel_rx,
            broker_event_channel_tx,
            agent_info_map,
        })
    }

    pub async fn start(mut self) -> Result<(), Box<dyn Error>> {
        self.cti_client.connect().await;

        tokio::spawn(async move {
            let tcp_acceptor = TCPAcceptor::new().await.unwrap();

            tcp_acceptor.accept().await.unwrap();
        });

        loop {
            match timeout(Duration::from_millis(10), self.cti_event_channel_rx.recv()).await {
                Ok(Some(event)) => match event {
                    // HeartBeat 요청 전송 시간 이벤트 수신
                    CTIEvent::TimeToHeartBeat => {
                        log::debug!("Received time to send heartbeat event.");
                        self.broker_event_channel_tx
                            .send(BrokerEvent::RequestHeartBeatReq)
                            .unwrap();
                    }
                    // 오류 이벤트 수신
                    CTIEvent::Error {
                        cti_server_host,
                        error_cause,
                    } => {
                        log::warn!(
                            "Received CTI Server error. cti_server_host: {}, error_cause: {}",
                            cti_server_host,
                            error_cause
                        );

                        // CTI 서버가 이중화 넘어가는데 시간이 소요됨
                        thread::sleep(Duration::from_millis(500));
                        self.is_active = !self.is_active;
                        self.cti_client = CTIClient::new(
                            self.is_active,
                            self.cti_event_channel_tx.clone(),
                            self.broker_event_channel_rx.resubscribe(),
                        )
                        .await?;
                        self.cti_client.connect().await;
                    }
                    // CTI 메시지 수신
                    CTIEvent::Recevied {
                        cti_server_host,
                        message_type,
                        mut data,
                    } => {
                        log::debug!(
                            "Received CTI event. cti_server_host: {}, message_type: {:?}, data: {:?}",
                            cti_server_host,
                            message_type, data
                        );
                        // 메시지 역직렬화
                        match message_type {
                            // OPEN_CONF 메시지 수신
                            MessageType::OPEN_CONF => {
                                let (_, open_conf) = OpenConf::deserialize(&mut data);
                                log::info!("{:?}", open_conf);
                            }
                            // AGENT_TEAM_CONFIG_EVENT 메시지 수신
                            MessageType::AGENT_TEAM_CONFIG_EVENT => {
                                let (_, agent_team_config_event) =
                                    AgentTeamConfigEvent::deserialize(&mut data);
                                log::info!("{:?}", agent_team_config_event);

                                // ATCAgent의 상태를 CTI 서버에 요청한다
                                agent_team_config_event.agents.iter().for_each(
                                    |agent| match &agent.agent_id {
                                        Some(agent_id) => {
                                            self.broker_event_channel_tx
                                                .send(BrokerEvent::RequestAgentStateEvent {
                                                    peripheral_id: agent_team_config_event
                                                        .peripheral_id,
                                                    agent_id: agent_id.data.clone(),
                                                })
                                                .unwrap();

                                            let agent_state =
                                                agent.agent_state.clone().unwrap().data;
                                            let state_duration = SystemTime::now()
                                                .duration_since(UNIX_EPOCH)
                                                .unwrap()
                                                .as_secs();
                                            let state_duration = state_duration
                                                - agent.state_duration.clone().unwrap().data as u64;

                                            match self.agent_info_map.get_mut(&agent_id.data) {
                                                Some(agent_info) => {
                                                    agent_info.agent_state = agent_state;
                                                    agent_info.state_duration = state_duration;

                                                    // 상담직원 이벤트 전송
                                                    Self::broadcast_agent_info(
                                                        self.broker_event_channel_tx.clone(),
                                                        agent_info.clone(),
                                                    );
                                                }
                                                None => {
                                                    let agent_info = AgentInfo {
                                                        icm_agent_id: 0,
                                                        agent_id: agent_id.data.clone(),
                                                        agent_state,
                                                        state_duration,
                                                        reason_code: 0,
                                                        skill_group_id: 0,
                                                        direction: 0,
                                                        agent_extension: "".to_string(),
                                                    };

                                                    self.agent_info_map.insert(
                                                        agent_id.data.clone(),
                                                        agent_info.clone(),
                                                    );

                                                    // 상담직원 이벤트 전송
                                                    Self::broadcast_agent_info(
                                                        self.broker_event_channel_tx.clone(),
                                                        agent_info,
                                                    );
                                                }
                                            };
                                        }
                                        None => {}
                                    },
                                );
                            }
                            // QUERY_AGENT_STATE_CONF 메시지 수신
                            MessageType::QUERY_AGENT_STATE_CONF => {
                                let (_, query_agent_state_conf) =
                                    QueryAgentStateConf::deserialize(&mut data);
                                log::info!("{:?}", query_agent_state_conf);

                                let agent_id = query_agent_state_conf.agent_id.unwrap().data;
                                let agent_state = query_agent_state_conf.agent_state;
                                let icm_agent_id = query_agent_state_conf.icm_agent_id;
                                let skill_group_id =
                                    query_agent_state_conf.skill_group_id.unwrap().data;
                                let agent_extension =
                                    query_agent_state_conf.agent_extension.unwrap().data;

                                match self.agent_info_map.get_mut(&agent_id) {
                                    Some(agent_info) => {
                                        agent_info.agent_state = agent_state;
                                        agent_info.skill_group_id = skill_group_id as u16;
                                        agent_info.icm_agent_id = icm_agent_id;
                                        agent_info.agent_extension = agent_extension;

                                        // 상담직원 이벤트 전송
                                        Self::broadcast_agent_info(
                                            self.broker_event_channel_tx.clone(),
                                            agent_info.clone(),
                                        );
                                    }
                                    None => {}
                                };
                            }
                            // 처리되지 않은 메시지 수신
                            message_type => {
                                log::info!(
                                    "Received CTI message. message_type: {:?}",
                                    message_type
                                );
                            }
                        }
                    }
                },
                Ok(None) => {}
                Err(_) => {}
            };
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    ///
    /// 상담직원 상태를 브로커 채널에 전송한다
    ///
    fn broadcast_agent_info(
        broker_event_channel_tx: broadcast::Sender<BrokerEvent>,
        agent_info: AgentInfo,
    ) {
        broker_event_channel_tx
            .send(BrokerEvent::BroadCastAgentState { agent_info })
            .unwrap();
    }
}
