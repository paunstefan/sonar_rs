use std::error::Error;
use std::io::Read;
use std::net::{SocketAddr, TcpListener, UdpSocket};
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread::{self};
use std::time::Duration;

use sg90_pi::SG90;
use sonar_common::*;

const CMD_SIZE: usize = 8;

fn main() -> Result<(), Box<dyn Error>> {
    let (tx, rx): (Sender<CmdData>, Receiver<CmdData>) = mpsc::channel();

    thread::spawn(move || {
        worker_thread(rx);
    });

    let listener = TcpListener::bind("0.0.0.0:1111")?;

    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                println!("New connection: {}", stream.peer_addr()?);
                tx.send(CmdData::PeerAddr(stream.peer_addr()?))?;
                loop {
                    let mut buffer = [0u8; CMD_SIZE];

                    if stream.read_exact(&mut buffer).is_err() {
                        tx.send(CmdData::Reset)?;
                        println!("Connection ended");
                        break;
                    }

                    let received: CmdData = bincode::deserialize(&buffer[..])?;

                    println!("{:?}", received);
                    // Command will be processed in the worker thread
                    tx.send(received)?;
                }
            }
            Err(e) => {
                tx.send(CmdData::Reset)?;
                eprintln!("Error {}", e);
            }
        }
    }

    Ok(())
}

#[derive(Debug, PartialEq)]
struct ServoState {
    fov: ScanFov,
    status: Status,
    angle: i32,
    dir: Direction,
}

#[derive(Debug, PartialEq, Copy, Clone)]
enum Direction {
    Ascending,
    Descending,
}

fn worker_thread(rx: Receiver<CmdData>) {
    let mut servo = SG90::new(rppal::pwm::Channel::Pwm0, 0.0).unwrap();
    let mut state = ServoState {
        fov: ScanFov::Wide,
        status: Status::Stop,
        angle: 0,
        dir: Direction::Ascending,
    };
    let socket = UdpSocket::bind("0.0.0.0:2222").unwrap();
    let mut peer_addr: Option<SocketAddr> = None;

    loop {
        if let Ok(new_cmd) = rx.try_recv() {
            match new_cmd {
                CmdData::FoV(f) => state.fov = f,
                CmdData::Operation(op) => state.status = op,
                CmdData::Reset => {
                    state.fov = ScanFov::Wide;
                    state.status = Status::Stop;
                    state.angle = 0;
                    state.dir = Direction::Ascending;
                    if servo.set_angle_deg(state.angle).is_err() {
                        eprintln!("Setting servo failed.");
                    }
                }
                CmdData::PeerAddr(addr) => {
                    peer_addr = Some(SocketAddr::new(addr.ip(), 1122));
                }
            }
        }

        if state.status == Status::Start {
            let (new_angle, new_dir) = next_angle(&state);
            state.angle = new_angle;
            state.dir = new_dir;

            if servo.set_angle_deg(state.angle).is_err() {
                eprintln!("Setting servo failed.");
            }
            if let Some(peer) = peer_addr {
                let data = bincode::serialize(&SensorData { angle: state.angle }).unwrap();
                socket.send_to(&data, peer).unwrap();
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

fn next_angle(current: &ServoState) -> (i32, Direction) {
    // For when FoV changes while angle is out of
    // new FoV's range
    if current.angle < current.fov.angle().0 {
        return (current.fov.angle().0, Direction::Ascending);
    } else if current.angle > current.fov.angle().1 {
        return (current.fov.angle().1, Direction::Descending);
    }

    let mut ret: (i32, Direction) = (current.angle, current.dir);

    match current.dir {
        Direction::Ascending => {
            ret.0 = current.angle + 1;
            if ret.0 == current.fov.angle().1 {
                ret.1 = Direction::Descending;
            }
        }
        Direction::Descending => {
            ret.0 = current.angle - 1;
            if ret.0 == current.fov.angle().0 {
                ret.1 = Direction::Ascending;
            }
        }
    }
    ret
}

// Crate will be compiled for RPi so `cargo test` will work only there
// for cross compilation comment the config.toml file to run tests
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_angle_change_simple() {
        let mut state = ServoState {
            fov: ScanFov::Wide,
            status: Status::Stop,
            angle: 0,
            dir: Direction::Ascending,
        };

        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(1, state.angle);

        let mut state = ServoState {
            fov: ScanFov::Narrow,
            status: Status::Stop,
            angle: -12,
            dir: Direction::Descending,
        };

        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(-13, state.angle);
    }

    #[test]
    fn test_fov_change_simple() {
        let mut state = ServoState {
            fov: ScanFov::Narrow,
            status: Status::Stop,
            angle: -50,
            dir: Direction::Ascending,
        };

        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(-45, state.angle);

        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(-44, state.angle);

        let mut state = ServoState {
            fov: ScanFov::Narrow,
            status: Status::Stop,
            angle: 60,
            dir: Direction::Ascending,
        };

        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(45, state.angle);

        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(44, state.angle);
    }

    #[test]
    fn test_angle_change_limit() {
        let mut state = ServoState {
            fov: ScanFov::Wide,
            status: Status::Stop,
            angle: 88,
            dir: Direction::Ascending,
        };
        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(89, state.angle);
        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(90, state.angle);
        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(89, state.angle);
    }

    #[test]
    fn test_angle_change_limit2() {
        let mut state = ServoState {
            fov: ScanFov::Narrow,
            status: Status::Stop,
            angle: -44,
            dir: Direction::Descending,
        };
        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(-45, state.angle);
        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(-44, state.angle);
        let (a, d) = next_angle(&state);
        state.angle = a;
        state.dir = d;

        assert_eq!(-43, state.angle);
    }
}
