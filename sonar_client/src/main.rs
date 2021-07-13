use std::error::Error;
use std::net::UdpSocket;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;

use sonar_common::*;

use fltk::{app, button::*, dialog::*, input::Input, output::*, prelude::*, window::Window};

mod state_machine;

use state_machine::*;

// TODO: drawing

#[derive(Debug)]
enum WorkerCommand {
    Connect(String),
    Disconnect,
    SendCmd(CmdData),
}

fn worker_thread(rx: Receiver<WorkerCommand>, gui_tx: Sender<GuiStatus>) {
    let mut sm = ConnectionMachine::new();

    loop {
        let (new_sm, new_gui_msg) = match rx.recv().unwrap() {
            WorkerCommand::Connect(addr) => sm.connect(addr),
            WorkerCommand::Disconnect => sm.disconnect(),
            WorkerCommand::SendCmd(cmd) => sm.send_data(cmd),
        };
        sm = new_sm;

        if gui_tx.send(new_gui_msg).is_err() {
            eprintln!("Sending status to GUI failed");
        }
    }
}

#[derive(Debug)]
struct SensorState {
    connection: bool,
    status: Status,
}

impl SensorState {
    fn ready(&self) -> bool {
        if self.connection && self.status == Status::Start {
            return true;
        }
        false
    }
}

#[allow(clippy::unnecessary_unwrap)]
fn main() -> Result<(), Box<dyn Error>> {
    // For sending commands to worker
    let (tx, rx): (Sender<WorkerCommand>, Receiver<WorkerCommand>) = mpsc::channel();
    // For receiving status
    let (gui_tx, gui_rx): (Sender<GuiStatus>, Receiver<GuiStatus>) = mpsc::channel();

    thread::spawn(move || {
        worker_thread(rx, gui_tx);
    });

    std::panic::set_hook(Box::new(|info| message(200, 200, &info.to_string())));

    let app = app::App::default().with_scheme(app::Scheme::Gtk);

    let mut wind = Window::default()
        .with_size(600, 700)
        .center_screen()
        .with_label("sonar_rs");

    let addr_inp = Input::default()
        .with_size(180, 30)
        .with_pos(80, 20)
        .with_label("Address");

    let mut but_conn = Button::default()
        .with_size(80, 30)
        .with_pos(280, 20)
        .with_label("Connect");

    let conn_sender = tx.clone();

    but_conn.set_callback(move |_| {
        let addr = addr_inp.value().trim().to_string();
        conn_sender.send(WorkerCommand::Connect(addr)).unwrap();
    });

    let mut butt_start = Button::default()
        .with_size(80, 30)
        .with_pos(20, 70)
        .with_label("START");

    let start_sender = tx.clone();

    butt_start.set_callback(move |_| {
        start_sender
            .send(WorkerCommand::SendCmd(CmdData::Operation(Status::Start)))
            .unwrap();
    });

    let mut butt_stop = Button::default()
        .with_size(80, 30)
        .with_pos(120, 70)
        .with_label("STOP");

    let stop_sender = tx.clone();

    butt_stop.set_callback(move |_| {
        stop_sender
            .send(WorkerCommand::SendCmd(CmdData::Operation(Status::Stop)))
            .unwrap();
    });

    let mut butt_wide = Button::default()
        .with_size(80, 30)
        .with_pos(20, 120)
        .with_label("Wide");

    let wide_sender = tx.clone();
    butt_wide.set_callback(move |_| {
        wide_sender
            .send(WorkerCommand::SendCmd(CmdData::FoV(ScanFov::Wide)))
            .unwrap();
    });

    let mut butt_narrow = Button::default()
        .with_size(80, 30)
        .with_pos(120, 120)
        .with_label("Narrow");

    let narrow_sender = tx.clone();

    butt_narrow.set_callback(move |_| {
        narrow_sender
            .send(WorkerCommand::SendCmd(CmdData::FoV(ScanFov::Narrow)))
            .unwrap();
    });

    let mut butt_disconnect = Button::default()
        .with_size(90, 30)
        .with_pos(20, 170)
        .with_label("Disconnect");

    let disconnect_sender = tx;

    butt_disconnect.set_callback(move |_| {
        disconnect_sender.send(WorkerCommand::Disconnect).unwrap();
    });

    let mut conn_text = Output::new(400, 20, 180, 30, "");
    conn_text.set_value("Disconnected");

    let mut status_text = Output::new(400, 70, 180, 30, "");
    status_text.set_value(":)");

    let mut fov_text = Output::new(400, 120, 180, 30, "");

    wind.make_resizable(true);
    wind.end();
    wind.show();

    let socket = UdpSocket::bind("0.0.0.0:1122").unwrap();
    let mut state = SensorState {
        connection: false,
        status: Status::Stop,
    };

    while app.wait() {
        let stat = gui_rx.try_recv();

        if stat.is_ok() {
            match stat.unwrap() {
                GuiStatus::Connected(addr) => {
                    fov_text.set_value("Wide");
                    conn_text.set_value(&addr);
                    status_text.set_value("Stop");
                    state.connection = true;
                }
                GuiStatus::Disconnected => {
                    fov_text.set_value("");
                    status_text.set_value("");
                    conn_text.set_value("Disconnected");
                    state.connection = false;
                }
                GuiStatus::ErrorConnecting => conn_text.set_value("Error connecting"),
                GuiStatus::Sent(data) => match data {
                    CmdData::FoV(fov) => fov_text.set_value(&format!("{:?}", fov)),
                    CmdData::Operation(status) => {
                        state.status = status;
                        status_text.set_value(&format!("{:?}", status))
                    }
                    _ => {}
                },
                GuiStatus::ErrorSending => conn_text.set_value("Error sending data"),
            }
        }

        if state.ready() {
            let mut buf = [0; 4];
            let (amt, _) = socket.recv_from(&mut buf)?;
            if amt == 4 {
                let received: SensorData = bincode::deserialize(&buf[..])?;
                println!("{:?}", received);
            }
        }
        wind.redraw();
        app::sleep(0.016);
    }

    Ok(())
}
