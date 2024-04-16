// #![allow(dead_code)]
use bytes::Bytes;

use std::{
    collections::HashMap,
    fmt,
    sync::{Arc, Mutex},
    thread,
    time::{Duration, SystemTime},
};

use tracing::{error, info};

use crate::{command::Command, util::hex_to_bytes};

use super::data_item::DataItem;

pub mod connection;

use connection::*;
use ResponseType::*;

const EMPTY_RDB : &str = "524544495330303131fa0972656469732d76657205372e322e30fa0a72656469732d62697473c040fa056374696d65c26d08bc65fa08757365642d6d656dc2b0c41000fa08616f662d62617365c000fff06e3bfec0ff5aa2";

#[derive(PartialEq, Clone, Copy)]
pub enum Role {
    Master,
    Slave,
}

impl fmt::Display for Role {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Role::Master => write!(f, "master"),
            Role::Slave => write!(f, "slave"),
        }
    }
}

pub struct ReplicationConfig {
    role: Role,
    master_replid: String,
    master_repl_offset: u64,
}

impl ReplicationConfig {
    pub fn new(role: Role) -> Self {
        ReplicationConfig {
            role,
            master_replid: "8371b4fb1155b71f4a04d3e1bc3e18c4a990aeeb".to_owned(),
            master_repl_offset: 0,
        }
    }
}

impl fmt::Display for ReplicationConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "role:{}\r\nmaster_replid:{}\r\nmaster_repl_offset:{}",
            self.role, self.master_replid, self.master_repl_offset
        )
    }
}

pub struct Core {
    storage_engine: Arc<Mutex<HashMap<String, DataItem<String>>>>,
    replicas: Arc<Mutex<Vec<Connection>>>,
}

impl Core {
    pub fn new() -> Self {
        Core {
            storage_engine: Arc::new(Mutex::new(HashMap::new())),
            replicas: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn set(&self, key: String, value: String, ttl: Option<SystemTime>) {
        self.storage_engine
            .lock()
            .unwrap()
            .insert(key, DataItem::new(value, ttl));
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.storage_engine
            .lock()
            .unwrap()
            .get(key)?
            .expired_or_return()
    }
}

pub struct RedisStore {
    address: String,
    core: Core,
    repli_config: ReplicationConfig,
}

impl RedisStore {
    pub fn new(role: Role, address: String) -> Self {
        RedisStore {
            address: address,
            core: Core::new(),
            repli_config: ReplicationConfig::new(role),
        }
    }

    pub fn set(&self, key: String, value: String, ttl: Option<SystemTime>) {
        self.core.set(key, value, ttl);
    }

    pub fn get(&self, key: &str) -> Option<String> {
        self.core.get(key)
        // self.data.lock().unwrap().get(key)?.expired_or_return()
    }
    fn apply_rdb_file(&self, _rdb_file: Bytes) {}

    fn replica_propagation(&self, cmd: Command) {
        if self.repli_config.role == Role::Master {
            let mut guard = self.core.replicas.lock().unwrap();
            for replica in guard.iter_mut() {
                info!("replicate {}", cmd.to_string());
                replica.write_request(cmd.to_string().as_str());
            }
        }
    }
}

pub fn start_replicate(store: Arc<RedisStore>, mut conn: Connection) {
    info!("Connected to master, sending PING");
    conn.write_request("PING");
    assert_eq!(conn.read_response().unwrap(), "PONG", "expect PONG");

    info!("Sending REPLCONF-1");
    let port = store.address.split(":").collect::<Vec<&str>>()[1];
    conn.write_request(format!("REPLCONF listening-port {}", port).as_str());
    assert_eq!(conn.read_response().unwrap(), "OK", "expect OK");

    info!("Sending REPLCONF-2");
    conn.write_request("REPLCONF capa psync2");
    assert_eq!(conn.read_response().unwrap(), "OK", "expect OK");

    info!("Sending PSYNC");
    conn.write_request("PSYNC ? -1");
    conn.read_response().unwrap();

    let rdb_file = conn.read_response().unwrap();
    store.apply_rdb_file(rdb_file.into());

    info!("Replication started");
    let store = Arc::clone(&store);
    thread::spawn(move || loop {
        let cmd = conn.read_request().unwrap();
        info!("Slave received command from Master: {}", cmd.to_string());
        match cmd {
            Command::REPLCONF(_) => {
                conn.write_response(RESPArray("REPLCONF ACK 0"));
            }
            Command::SET(key, value, ttl) => {
                let ttl = match ttl {
                    Some(ttl) => SystemTime::UNIX_EPOCH.checked_add(Duration::from_millis(ttl)),
                    None => None,
                };
                store.set(key, value, ttl);
            }
            _ => {
                error!("Unexpected command from master: {}", cmd.to_string());
            }
        }
    });
}

pub fn handle_connection(store: Arc<RedisStore>, mut conn: Connection) {
    loop {
        let cmd = conn.read_request().unwrap();
        match cmd {
            Command::PING => {
                conn.write_response(SimpleString("PONG"));
            }
            Command::ECHO(content) => {
                conn.write_response(SimpleString(&content));
            }
            Command::REPLCONF(_content) => {
                conn.write_response(SimpleString("OK"));
            }
            Command::PSYNC(_id, _offset) => {
                conn.write_response(SimpleString(
                    format!("FULLRESYNC {} 0", store.repli_config.master_replid).as_str(),
                ));

                let empty_rdb = hex_to_bytes(EMPTY_RDB).unwrap();
                conn.write_response(RdbFile(empty_rdb.into()));

                // TODO: assume there is only one stream for a replica
                // TODO: This connection no longer accepts new commands?
                store.core.replicas.lock().unwrap().push(conn);
                return;
            }
            Command::INFO(section) => match section.as_str() {
                "replication" => {
                    conn.write_response(BulkString(Some(store.repli_config.to_string())));
                }
                &_ => {
                    conn.write_response(SimpleError("ERR unknown section"));
                }
            },
            Command::SET(key, value, ttl) => {
                let ttl = match ttl {
                    Some(ttl) => SystemTime::now().checked_add(Duration::from_millis(ttl)),
                    None => None,
                };

                store.set(key.clone(), value.clone(), ttl);
                conn.write_response(SimpleString("OK"));
                store.replica_propagation(Command::SET(
                    key,
                    value,
                    ttl.map(|t| {
                        t.duration_since(SystemTime::UNIX_EPOCH)
                            .unwrap()
                            .as_millis() as u64
                    }),
                ));
            }
            Command::GET(key) => {
                if let Some(value) = store.get(&key) {
                    conn.write_response(BulkString(Some(value)));
                } else {
                    conn.write_response(BulkString(None));
                }
            }
        }
    }
}
