/**
Forked-off from https://github.com/AndrewWestberg/cncli/ on 2020-11-30
© 2020 Andrew Westberg licensed under Apache-2.0

Re-licensed under GPLv3 or LGPLv3
© 2020 PERLUR Group

SPDX-License-Identifier: GPL-3.0-only OR LGPL-3.0-only

*/

use std::collections::BTreeMap;

use log::debug;
use serde_cbor::{de, ser, Value};

use crate::{Agency, Protocol};

const PROTOCOL_VERSION_1: i128 = 0x01;
const PROTOCOL_VERSION_2: i128 = 0x02;
const PROTOCOL_VERSION_SHELLEY: i128 = 0x03;
const PROTOCOL_VERSION_SHELLEY2: i128 = 0x04;
const PROTOCOL_VERSION_ALLEGRA: i128 = 0x05;
const MIN_PROTOCOL_VERSION: i128 = PROTOCOL_VERSION_ALLEGRA;

const MSG_ACCEPT_VERSION_MSG_ID: i128 = 1;

#[derive(Debug)]
pub enum State {
    Propose,
    Confirm,
    Done,
}

pub struct HandshakeProtocol {
    network_magic: u32,
    state: State,
    result: Option<Result<String, String>>,
}

impl HandshakeProtocol {
    pub fn new(network_magic: u32) -> Self {
        HandshakeProtocol {
            network_magic,
            state: State::Propose,
            result: None,
        }
    }

    // Serialize cbor for MsgProposeVersions
    //
    // Create the byte representation of MsgProposeVersions for sending to the server
    fn msg_propose_versions(&self, network_magic: u32) -> Vec<u8> {
        let mut payload_map: BTreeMap<Value, Value> = BTreeMap::new();
        payload_map.insert(Value::Integer(PROTOCOL_VERSION_1), Value::Integer(network_magic as i128));
        payload_map.insert(Value::Integer(PROTOCOL_VERSION_2), Value::Integer(network_magic as i128));
        payload_map.insert(Value::Integer(PROTOCOL_VERSION_SHELLEY), Value::Integer(network_magic as i128));
        payload_map.insert(Value::Integer(PROTOCOL_VERSION_SHELLEY2), Value::Array(vec![Value::Integer(network_magic as i128), Value::Bool(false)]));
        payload_map.insert(Value::Integer(PROTOCOL_VERSION_ALLEGRA), Value::Array(vec![Value::Integer(network_magic as i128), Value::Bool(false)]));

        let message = Value::Array(vec![
            Value::Integer(0), // message_id
            Value::Map(payload_map)
        ]);

        ser::to_vec_packed(&message).unwrap()
    }

    // Search through the cbor values until we find a Text value.
    fn find_error_message(&self, cbor_value: &Value) -> Result<String, ()> {
        match cbor_value {
            Value::Text(cbor_text) => {
                return Ok(cbor_text.to_owned());
            }
            Value::Array(cbor_array) => {
                for value in cbor_array {
                    let result = self.find_error_message(value);
                    if result.is_ok() {
                        return result;
                    }
                }
            }
            _ => {}
        }
        return Err(());
    }
}

impl Protocol for HandshakeProtocol {
    fn protocol_id(&self) -> u16 {
        return 0x0000u16;
    }

    fn result(&self) -> Result<String, String> {
        self.result.clone().unwrap()
    }

    fn get_agency(&self) -> Agency {
        return match self.state {
            State::Propose => { Agency::Client }
            State::Confirm => { Agency::Server }
            State::Done => { Agency::None }
        };
    }

    fn get_state(&self) -> String {
        format!("{:?}", self.state)
    }

    fn send_data(&mut self) -> Option<Vec<u8>> {
        return match self.state {
            State::Propose => {
                debug!("HandshakeProtocol::State::Propose");
                let payload = self.msg_propose_versions(self.network_magic);
                self.state = State::Confirm;
                Some(payload)
            }
            State::Confirm => {
                debug!("HandshakeProtocol::State::Confirm");
                None
            }
            State::Done => {
                debug!("HandshakeProtocol::State::Done");
                None
            }
        };
    }

    fn receive_data(&mut self, data: Vec<u8>) {
        let confirm: Value = de::from_slice(&data[..]).unwrap();
        debug!("Confirm: {:?}", &confirm);
        match &confirm {
            Value::Array(confirm_vec) => {
                match confirm_vec.get(0) {
                    Some(msg_type) => {
                        match msg_type {
                            Value::Integer(msg_type_int) => {
                                if *msg_type_int == MSG_ACCEPT_VERSION_MSG_ID {
                                    match confirm_vec.get(1) {
                                        Some(accepted_protocol_value) => {
                                            match accepted_protocol_value {
                                                Value::Integer(accepted_protocol) => {
                                                    if *accepted_protocol < MIN_PROTOCOL_VERSION {
                                                        self.result = Some(Err(format!("Expected protocol version {}, but was {}", MIN_PROTOCOL_VERSION, accepted_protocol)));
                                                    } else {
                                                        match confirm_vec.get(2) {
                                                            Some(accepted_vec_value) => {
                                                                match accepted_vec_value {
                                                                    Value::Array(accepted_vec) => {
                                                                        match accepted_vec.get(0) {
                                                                            Some(accepted_magic_value) => {
                                                                                match accepted_magic_value {
                                                                                    Value::Integer(accepted_magic) => {
                                                                                        if *accepted_magic == self.network_magic as i128 {
                                                                                            self.result = Some(Ok(hex::encode(data)));
                                                                                        } else {
                                                                                            self.result = Some(Err(format!("Expected network magic {}, but was {}", self.network_magic, accepted_magic)));
                                                                                        }
                                                                                    }
                                                                                    _ => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                                                                                }
                                                                            }
                                                                            None => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                                                                        }
                                                                    }
                                                                    _ => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                                                                }
                                                            }
                                                            None => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                                                        }
                                                    }
                                                }
                                                _ => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                                            }
                                        }
                                        None => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                                    }
                                } else {
                                    match self.find_error_message(&confirm) {
                                        Ok(error_message) => {
                                            self.result = Some(Err(error_message));
                                        }
                                        Err(_) => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                                    }
                                }
                            }
                            _ => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                        }
                    }
                    None => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
                }
            }
            _ => { self.result = Some(Err(format!("Unable to parse payload error! {}", hex::encode(data)))); }
        }

        debug!("HandshakeProtocol::State::Done");
        self.state = State::Done
    }
}
