use sonar_common::*;
use std::io::prelude::*;
use std::net::{Shutdown, TcpStream};

#[derive(Debug)]
enum State {
    Connected(TcpStream),
    Disconnected,
}

#[derive(Debug)]
pub struct ConnectionMachine {
    state: State,
}

// To be sent to GUI to signal state
#[derive(Debug)]
pub enum GuiStatus {
    Connected(String),
    Disconnected,
    ErrorConnecting,
    Sent(CmdData),
    ErrorSending,
}

#[allow(clippy::new_without_default)]
impl ConnectionMachine {
    pub fn new() -> Self {
        ConnectionMachine {
            state: State::Disconnected,
        }
    }

    pub fn connect(mut self, addr: String) -> (Self, GuiStatus) {
        // First we check the current state of the state machine
        let (new_state, gui_msg) = match self.state {
            // For the Disconnected state, match the connection result to get next state
            // and message for the GUI
            State::Disconnected => match TcpStream::connect(&addr) {
                Ok(conn) => (State::Connected(conn), GuiStatus::Connected(addr)),
                Err(_) => (State::Disconnected, GuiStatus::ErrorConnecting),
            },
            _ => (self.state, GuiStatus::Disconnected),
        };
        self.state = new_state;

        (self, gui_msg)
    }

    pub fn disconnect(mut self) -> (Self, GuiStatus) {
        let (new_state, gui_msg) = match self.state {
            State::Connected(socket) => {
                socket.shutdown(Shutdown::Both).unwrap();
                (State::Disconnected, GuiStatus::Disconnected)
            }
            _ => (self.state, GuiStatus::Disconnected),
        };
        self.state = new_state;

        (self, gui_msg)
    }

    pub fn send_data(mut self, data: CmdData) -> (Self, GuiStatus) {
        let gui_msg = match self.state {
            State::Connected(mut socket) => {
                // If serialization succeeds, write the data to the socket
                let gui_status =
                    match bincode::serialize(&data).map(|encoded| socket.write_all(&encoded)) {
                        Ok(_) => GuiStatus::Sent(data),
                        Err(_) => GuiStatus::ErrorSending,
                    };

                self.state = State::Connected(socket);
                gui_status
            }
            State::Disconnected => GuiStatus::ErrorSending,
        };

        (self, gui_msg)
    }
}
