use std::{collections::HashMap, error::Error, thread, time::Duration};

use tokio::{
    sync::{broadcast, mpsc},
    time::timeout,
};
use uuid::Uuid;

use crate::{
    cisco::{
        client_event::agent_state_event::AgentStateEvent,
        control::query_agent_state_conf::QueryAgentStateConf, session::OpenConf,
        supervisor::agent_team_config_event::AgentTeamConfigEvent, Deserializable, MessageType,
    },
    ctm::cti_client::CTIClient,
    event::{broker_event::BrokerEvent, client_event::ClientEvent, cti_event::CTIEvent},
};

use super::{
    acceptor::{tcp_acceptor::TCPAcceptor, websocket_acceptor::WebsocketAcceptor, Acceptor},
    agent_info::AgentInfo,
};

pub struct CTM {
    is_active: bool,
    cti_client: CTIClient,
    cti_event_channel_rx: mpsc::Receiver<CTIEvent>,
    cti_event_channel_tx: mpsc::Sender<CTIEvent>,
    broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
    broker_event_channel_tx: broadcast::Sender<BrokerEvent>,
    client_event_channel_rx: mpsc::Receiver<ClientEvent>,
    client_event_channel_tx: mpsc::Sender<ClientEvent>,
    agent_info_map: HashMap<String, AgentInfo>,
}

impl CTM {
    ///
    /// 새로운 CTM 구조체 생성
    /// 
    pub async fn new() -> Result<Self, Box<dyn Error>> {
        let is_active = true;
        let (cti_event_channel_tx, cti_event_channel_rx) = mpsc::channel::<CTIEvent>(1_024);
        let (broker_event_channel_tx, broker_event_channel_rx) =
            broadcast::channel::<BrokerEvent>(1_024);
        let (client_event_channel_tx, client_event_channel_rx) =
            mpsc::channel::<ClientEvent>(4_096);

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
            client_event_channel_rx,
            client_event_channel_tx,
            agent_info_map,
        })
    }

    ///
    /// CTM 서버 실행
    /// 
    pub async fn start(mut self) -> Result<(), Box<dyn Error>> {
        self.cti_client.connect().await;

        let mut acceptors: Vec<Box<dyn Acceptor>> = Vec::new();

        // TCP Acceptor 생성
        if dotenv::var("TCP_ACCEPTOR_ENABLED")
            .unwrap_or("false".to_string())
            .parse::<bool>()
            .unwrap_or(false)
        {
            let broker_event_channel_rx = self.broker_event_channel_rx.resubscribe();
            let client_event_channel_tx = self.client_event_channel_tx.clone();

            match TCPAcceptor::new(broker_event_channel_rx, client_event_channel_tx).await {
                Ok(acceptor) => acceptors.push(Box::new(acceptor)),
                Err(_) => {}
            }
        }

        // 웹 소켓 Acceptor 생성
        if dotenv::var("WEBSOCKET_ACCEPTOR_ENABLED")
            .unwrap_or("false".to_string())
            .parse::<bool>()
            .unwrap_or(false)
        {
            let broker_event_channel_rx = self.broker_event_channel_rx.resubscribe();
            let client_event_channel_tx = self.client_event_channel_tx.clone();

            match WebsocketAcceptor::new(broker_event_channel_rx, client_event_channel_tx).await {
                Ok(acceptor) => acceptors.push(Box::new(acceptor)),
                Err(_) => {}
            }
        }

        // Acceptor 실행
        for acceptor in acceptors {
            tokio::spawn(async move {
                acceptor.accept().await.unwrap();
            });
        }

        loop {
            // CTI 이벤트 채널 데이터 수신
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
                                            let state_duration =
                                                agent.state_duration.clone().unwrap().data;

                                            match self.agent_info_map.get_mut(&agent_id.data) {
                                                Some(agent_info) => {
                                                    agent_info.set_agent_state(agent_state);
                                                    agent_info.set_state_duration(state_duration);

                                                    // 상담직원 이벤트 전송
                                                    Self::broadcast_agent_info(
                                                        None,
                                                        self.broker_event_channel_tx.clone(),
                                                        agent_info.clone(),
                                                    );
                                                }
                                                None => {
                                                    let mut agent_info =
                                                        AgentInfo::new(agent_id.clone().data);

                                                    agent_info.set_agent_state(agent_state);
                                                    agent_info.set_state_duration(state_duration);

                                                    self.agent_info_map.insert(
                                                        agent_id.data.clone(),
                                                        agent_info.clone(),
                                                    );

                                                    // 상담직원 이벤트 전송
                                                    Self::broadcast_agent_info(
                                                        None,
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
                                        agent_info.set_agent_state(agent_state);
                                        agent_info.set_skill_group_id(skill_group_id as u16);
                                        agent_info.set_icm_agent_id(icm_agent_id);
                                        agent_info.set_agent_extension(agent_extension);

                                        // 상담직원 이벤트 전송
                                        Self::broadcast_agent_info(
                                            None,
                                            self.broker_event_channel_tx.clone(),
                                            agent_info.clone(),
                                        );
                                    }
                                    None => {}
                                };
                            }
                            // AGENT_STATE_EVENT 메시지 수신
                            MessageType::AGENT_STATE_EVENT => {
                                let (_, agent_state_event) =
                                    AgentStateEvent::deserialize(&mut data);

                                log::info!("{:?}", agent_state_event);

                                let agent_id = agent_state_event.agent_id.unwrap().data;
                                let agent_state = agent_state_event.agent_state;
                                let icm_agent_id = agent_state_event.icm_agent_id;
                                let skill_group_id = agent_state_event.skill_group_id;
                                let agent_extension =
                                    agent_state_event.agent_extension.unwrap().data;
                                let direction = agent_state_event.direction.unwrap().data;
                                let reason_code = agent_state_event.event_reason_code;
                                let state_duration = agent_state_event.state_duration;

                                match self.agent_info_map.get_mut(&agent_id) {
                                    Some(agent_info) => {
                                        agent_info.set_agent_state(agent_state);
                                        agent_info.set_skill_group_id(skill_group_id as u16);
                                        agent_info.set_icm_agent_id(icm_agent_id);
                                        agent_info.set_agent_extension(agent_extension);
                                        agent_info.set_direction(direction);
                                        agent_info.set_reason_code(reason_code);
                                        agent_info.set_state_duration(state_duration);

                                        // 상담직원 이벤트 전송
                                        Self::broadcast_agent_info(
                                            None,
                                            self.broker_event_channel_tx.clone(),
                                            agent_info.clone(),
                                        );
                                    }
                                    None => {}
                                }
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

            // 클라이언트 이벤트 채널 수신
            match timeout(
                Duration::from_millis(10),
                self.client_event_channel_rx.recv(),
            )
            .await
            {
                Ok(Some(event)) => match event {
                    ClientEvent::Connect { id } => {
                        self.agent_info_map.iter().for_each(|(_, agent_info)| {
                            Self::broadcast_agent_info(
                                Some(id),
                                self.broker_event_channel_tx.clone(),
                                agent_info.clone(),
                            );
                        });
                    }
                    ClientEvent::Receive { data, id } => {
                        log::debug!("Client sent. id: {}, data: {:?}", id, data);
                    }
                    ClientEvent::Disconnect { id: _ } => {}
                },
                Ok(None) => {}
                Err(_) => {}
            }
        }

        #[allow(unreachable_code)]
        Ok(())
    }

    ///
    /// 상담직원 상태를 브로커 채널에 전송한다
    ///
    fn broadcast_agent_info(
        target_client_id: Option<Uuid>,
        broker_event_channel_tx: broadcast::Sender<BrokerEvent>,
        agent_info: AgentInfo,
    ) {
        broker_event_channel_tx
            .send(BrokerEvent::BroadCastAgentState {
                agent_info,
                client_id: target_client_id,
            })
            .unwrap();
        log::debug!("Broadcasted agent info event.");
    }
}
