use std::{error::Error, thread, time::Duration};

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

pub struct CTM {
    is_active: bool,
    cti_client: CTIClient,
    cti_event_channel_rx: mpsc::Receiver<CTIEvent>,
    cti_event_channel_tx: mpsc::Sender<CTIEvent>,
    broker_event_channel_rx: broadcast::Receiver<BrokerEvent>,
    broker_event_channel_tx: broadcast::Sender<BrokerEvent>,
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

        Ok(Self {
            is_active,
            cti_client,
            cti_event_channel_rx,
            cti_event_channel_tx,
            broker_event_channel_rx,
            broker_event_channel_tx,
        })
    }

    pub async fn start(mut self) -> Result<(), Box<dyn Error>> {
        self.cti_client.connect().await;

        loop {
            match timeout(Duration::from_millis(10), self.cti_event_channel_rx.recv()).await {
                Ok(Some(event)) => match event {
                    // 오류 이벤트 수신신
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
}
