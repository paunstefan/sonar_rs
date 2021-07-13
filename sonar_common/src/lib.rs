use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SensorData {
    pub angle: i32,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum CmdData {
    FoV(ScanFov),
    Operation(Status),
    PeerAddr(std::net::SocketAddr),
    Reset,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum ScanFov {
    Narrow,
    Wide,
}

impl ScanFov {
    pub fn angle(&self) -> (i32, i32) {
        match self {
            ScanFov::Narrow => (-45, 45),
            ScanFov::Wide => (-90, 90),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum Status {
    Start,
    Stop,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_cmd_serialize() {
        let d = CmdData::Operation(Status::Start);
        let encoded = bincode::serialize(&d).unwrap();

        println!("{}", encoded.len());

        let decoded: CmdData = bincode::deserialize(&encoded[..]).unwrap();

        println!("{:?}", decoded);

        assert_eq!(decoded, d);
    }
}
