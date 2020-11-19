/* =======================================================================
Rustic is a chess playing engine.
Copyright (C) 2019-2020, Marcel Vanthoor

Rustic is written in the Rust programming language. It is an original
work, not derived from any engine that came before it. However, it does
use a lot of concepts which are well-known and are in use by most if not
all classical alpha/beta-based chess engines.

Rustic is free software: you can redistribute it and/or modify it under
the terms of the GNU General Public License version 3 as published by
the Free Software Foundation.

Rustic is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.  See the GNU General Public License
for more details.

You should have received a copy of the GNU General Public License along
with this program.  If not, see <http://www.gnu.org/licenses/>.
======================================================================= */

// This file implements the XBoard communication module.

use super::{CommControl, CommReport, CommType, IComm};
use crate::{
    board::Board,
    defs::About,
    engine::defs::{ErrFatal, Information},
    misc::print,
};
use crossbeam_channel::{self, Sender};
use std::{
    io::{self},
    sync::{Arc, Mutex},
    thread::{self, JoinHandle},
};

// Input will be turned into a report, which wil be sent to the engine. The
// main engine thread will react accordingly.
#[derive(PartialEq, Clone)]
pub enum XBoardReport {
    // XBoard commands
    XBoard,
    ProtoVer(u8),
    Ping(isize),
    Accepted(String),
    SetBoard(String),
    Quit,

    // Custom commands
    Board,
    History,
    Eval,
    Help,

    // Empty or unknown command.
    Unknown(String),
}

// This struct is used to instantiate the Comm module.
pub struct XBoard {
    control_handle: Option<JoinHandle<()>>,
    report_handle: Option<JoinHandle<()>>,
    control_tx: Option<Sender<CommControl>>,
}

// Public functions
impl XBoard {
    // Create a new console.
    pub fn new() -> Self {
        Self {
            control_handle: None,
            report_handle: None,
            control_tx: None,
        }
    }
}

// Any communication module must implement the trait IComm.
impl IComm for XBoard {
    fn init(&mut self, report_tx: Sender<Information>, board: Arc<Mutex<Board>>) {
        // Start threads
        self.report_thread(report_tx);
        self.control_thread(board);
    }

    // The creator of the Comm module can use this function to send
    // messages or commands into the Control thread.
    fn send(&self, msg: CommControl) {
        if let Some(tx) = &self.control_tx {
            tx.send(msg).expect(ErrFatal::CHANNEL);
        }
    }

    // After the engine sends 'quit' to the control thread, it will call
    // wait_for_shutdown() and then wait here until shutdown is completed.
    fn wait_for_shutdown(&mut self) {
        if let Some(h) = self.report_handle.take() {
            h.join().expect(ErrFatal::THREAD);
        }

        if let Some(h) = self.control_handle.take() {
            h.join().expect(ErrFatal::THREAD);
        }
    }

    // This function just returns the name of the communication protocol.
    fn get_protocol_name(&self) -> &'static str {
        CommType::XBOARD
    }
}

// This block implements the Report and Control threads.
impl XBoard {
    // The Report thread sends incoming data to the engine thread.
    fn report_thread(&mut self, report_tx: Sender<Information>) {
        // Create thread-local variables
        let mut t_incoming_data = String::from("");
        let t_report_tx = report_tx; // Report sender

        // Actual thread creation.
        let report_handle = thread::spawn(move || {
            let mut quit = false;

            // Keep running as long as 'quit' is not detected.
            while !quit {
                // Get data from stdin.
                io::stdin()
                    .read_line(&mut t_incoming_data)
                    .expect(ErrFatal::READ_IO);

                // Create a report from the incoming data.
                let new_report = XBoard::create_report(&t_incoming_data);

                // Check if the created report is valid, so it is something
                // the engine will understand.
                if new_report.is_valid() {
                    // Send it to the engine thread.
                    t_report_tx
                        .send(Information::Comm(new_report.clone()))
                        .expect(ErrFatal::HANDLE);

                    // Terminate the reporting thread if "Quit" was detected.
                    quit = new_report == CommReport::XBoard(XBoardReport::Quit);
                }

                // Clear for next input
                t_incoming_data = String::from("");
            }
        });

        // Store the handle.
        self.report_handle = Some(report_handle);
    }

    // The control thread receives commands from the engine thread.
    fn control_thread(&mut self, board: Arc<Mutex<Board>>) {
        // Create an incoming channel for the control thread.
        let (control_tx, control_rx) = crossbeam_channel::unbounded::<CommControl>();

        // Create the control thread.
        let control_handle = thread::spawn(move || {
            let mut quit = false;
            let t_board = Arc::clone(&board);

            // Keep running as long as Quit is not received.
            while !quit {
                let control = control_rx.recv().expect(ErrFatal::CHANNEL);
                match control {
                    // Perform command as sent by the engine thread.
                    CommControl::Identify => XBoard::identify(),
                    CommControl::Ready => XBoard::ready(),
                    CommControl::Pong(n) => XBoard::pong(n),
                    CommControl::Message(m) => XBoard::message(m),
                    CommControl::Quit => quit = true,

                    // Custom prints for use in the console.
                    CommControl::PrintBoard => XBoard::print_board(&t_board),
                    CommControl::PrintHistory => XBoard::print_history(&t_board),

                    CommControl::PrintHelp => XBoard::print_help(),

                    // Ignore stuff the XBoard protocol doesn't need.
                    _ => (),
                }
            }
        });

        // Store handle and control sender.
        self.control_handle = Some(control_handle);
        self.control_tx = Some(control_tx);
    }
}

// Private functions for this module.
impl XBoard {
    // This function turns the incoming data into XBoardReports which the
    // engine is able to understand and react to.
    fn create_report(input: &str) -> CommReport {
        // Trim CR/LF so only the usable characters remain.
        let i = input.trim_end().to_string();

        // Convert to &str for matching the command.
        match i {
            // XBoard Commands
            cmd if cmd == "xboard" => CommReport::XBoard(XBoardReport::XBoard),
            cmd if cmd.starts_with("protover") => XBoard::parse_protover(&cmd),
            cmd if cmd.starts_with("ping") => XBoard::parse_ping(&cmd),
            cmd if cmd.starts_with("accepted") => XBoard::parse_accepted(&cmd),
            cmd if cmd.starts_with("setboard") => XBoard::parse_setboard(&cmd),
            cmd if cmd == "quit" || cmd == "exit" => CommReport::XBoard(XBoardReport::Quit),

            // Custom commands
            cmd if cmd == "board" => CommReport::XBoard(XBoardReport::Board),
            cmd if cmd == "history" => CommReport::XBoard(XBoardReport::History),
            cmd if cmd == "eval" => CommReport::XBoard(XBoardReport::Eval),
            cmd if cmd == "help" => CommReport::XBoard(XBoardReport::Help),

            // Everything else is ignored.
            _ => CommReport::XBoard(XBoardReport::Unknown(i)),
        }
    }
}

// Implements XBoard responses to send to the G(UI).
impl XBoard {
    fn parse_protover(cmd: &str) -> CommReport {
        enum Tokens {
            Nothing,
            ProtoVer,
        }

        let mut token = Tokens::Nothing;
        let mut report = CommReport::XBoard(XBoardReport::ProtoVer(0));
        let parts: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();

        for p in parts {
            match p {
                t if t == "protover" => token = Tokens::ProtoVer,
                _ => match token {
                    Tokens::Nothing => (),
                    Tokens::ProtoVer => {
                        let v = p.parse::<u8>().unwrap_or(0);
                        report = CommReport::XBoard(XBoardReport::ProtoVer(v));
                    }
                },
            }
        }
        report
    }

    fn parse_ping(cmd: &str) -> CommReport {
        enum Tokens {
            Nothing,
            Ping,
        }

        let mut token = Tokens::Nothing;
        let mut report = CommReport::XBoard(XBoardReport::Ping(0));
        let parts: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();

        for p in parts {
            match p {
                t if t == "ping" => token = Tokens::Ping,
                _ => match token {
                    Tokens::Nothing => (),
                    Tokens::Ping => {
                        let n = p.parse::<isize>().unwrap_or(0);
                        report = CommReport::XBoard(XBoardReport::Ping(n));
                    }
                },
            }
        }
        report
    }

    fn parse_accepted(cmd: &str) -> CommReport {
        enum Tokens {
            Nothing,
            Accepted,
        }

        let mut token = Tokens::Nothing;
        let mut report = CommReport::XBoard(XBoardReport::Unknown(String::from(cmd)));
        let parts: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();

        for p in parts {
            match p {
                t if t == "accepted" => token = Tokens::Accepted,
                _ => match token {
                    Tokens::Nothing => (),
                    Tokens::Accepted => report = CommReport::XBoard(XBoardReport::Accepted(p)),
                },
            }
        }
        report
    }

    fn parse_setboard(cmd: &str) -> CommReport {
        enum Tokens {
            Nothing,
            SetBoard,
        }

        let mut token = Tokens::Nothing;
        let mut fen = String::from("");
        let parts: Vec<String> = cmd.split_whitespace().map(|s| s.to_string()).collect();
        for p in parts {
            match p {
                t if t == "setboard" => token = Tokens::SetBoard,
                _ => match token {
                    Tokens::Nothing => (),
                    Tokens::SetBoard => {
                        fen.push_str(&p[..]);
                        fen.push(' ');
                    }
                },
            }
        }
        CommReport::XBoard(XBoardReport::SetBoard(fen.trim().to_string()))
    }
}

impl XBoard {
    fn identify() {
        println!("feature done=0");
        println!("feature myname=\"{} {}\"", About::ENGINE, About::VERSION);
        println!("feature ping=1");
        println!("feature setboard=1");
        println!("feature usermove=1");
        println!("feature debug=1");
        println!("feature sigint=0");
        println!("feature sigterm=0");
    }

    fn ready() {
        println!("feature done=1");
    }

    fn pong(n: isize) {
        println!("pong {}", n);
    }

    fn message(msg: String) {
        println!("{}", msg);
    }
}

// implements handling of custom commands. These are mostly used when using
// the XBoard protocol directly in a terminal window.
impl XBoard {
    fn print_board(board: &Arc<Mutex<Board>>) {
        print::position(&board.lock().expect(ErrFatal::LOCK), None);
    }

    fn print_history(board: &Arc<Mutex<Board>>) {
        let mtx_board = board.lock().expect(ErrFatal::LOCK);
        let length = mtx_board.history.len();

        if length == 0 {
            println!("No history available.");
        }

        for i in 0..length {
            let h = mtx_board.history.get_ref(i);
            println!("{:<3}| ply: {} {}", i, i + 1, h.as_string());
        }

        std::mem::drop(mtx_board);
    }

    fn print_help() {
        println!("The engine is in XBoard communication mode. It supports some custom");
        println!("non-XBoard commands to make use through a terminal window easier.");
        println!("These commands can also be very useful for debugging purposes.");
        println!();
        println!("Custom commands");
        println!("================================================================");
        println!("help      :   This help information.");
        println!("board     :   Print the current board state.");
        println!("history   :   Print a list of past board states.");
        println!("eval      :   Print evaluation for side to move.");
        println!("exit      :   Quit/Exit the engine.");
        println!();
    }
}
