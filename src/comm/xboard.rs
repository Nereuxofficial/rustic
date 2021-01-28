/* =======================================================================
Rustic is a chess playing engine.
Copyright (C) 2019-2021, Marcel Vanthoor
https://rustic-chess.org/

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

use super::{shared, CommControl, CommReport, CommType, IComm};
use crate::{
    board::Board,
    defs::About,
    engine::defs::{ErrFatal, Information, XBoardStat01},
    search::defs::SearchSummary,
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
    ProtoVer(u8),
    Ping(isize),
    Accepted(String),
    SetBoard(String),
    New,
    Question,
    Post,
    NoPost,
    Analyze,
    Dot,
    Exit, // Stop analyzing current position.
    Quit, // Completely shut down engine.

    // These commands will be sent to the engine, but it will ignore them;
    // either because no reply is required, or the functionality is not
    // supported by the engine at this time.
    XBoard,
    Random,
    Easy,
    Hard,

    // Custom commands
    Board,
    History,
    Legal,
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
                    CommControl::Empty => println!(),
                    CommControl::Identify => XBoard::identify(),
                    CommControl::Ready => XBoard::ready(),
                    CommControl::Pong(n) => XBoard::pong(n),
                    CommControl::Message(m) => XBoard::message(m),
                    CommControl::SearchSummary(s) => XBoard::search_summary(s),
                    CommControl::AnalyzeStat01(s) => XBoard::search_stat01(s),
                    CommControl::Quit => quit = true,

                    // Custom prints for use in the console.
                    CommControl::PrintBoard => shared::print_board(&t_board),
                    CommControl::PrintHistory => shared::print_history(&t_board),
                    CommControl::PrintEval(e) => shared::print_eval(e),
                    CommControl::PrintLegal(ml) => shared::print_legal(ml),
                    CommControl::PrintHelp => shared::print_help("XBoard"),

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

// Determine the command coming in from the GUI and parse it.
impl XBoard {
    // This function turns the incoming data into XBoardReports which the
    // engine is able to understand and react to.
    fn create_report(input: &str) -> CommReport {
        // Trim CR/LF so only the usable characters remain.
        let i = input.trim_end().to_string();

        // Convert to &str for matching the command.
        match i {
            // XBoard Commands
            cmd if cmd.starts_with("protover") => XBoard::parse_protover(&cmd),
            cmd if cmd.starts_with("ping") => XBoard::parse_ping(&cmd),
            cmd if cmd.starts_with("accepted") => XBoard::parse_accepted(&cmd),
            cmd if cmd.starts_with("setboard") => XBoard::parse_setboard(&cmd),
            cmd if cmd == "xboard" => CommReport::XBoard(XBoardReport::XBoard),
            cmd if cmd == "new" => CommReport::XBoard(XBoardReport::New),
            cmd if cmd == "?" => CommReport::XBoard(XBoardReport::Question),
            cmd if cmd == "post" => CommReport::XBoard(XBoardReport::Post),
            cmd if cmd == "nopost" => CommReport::XBoard(XBoardReport::NoPost),
            cmd if cmd == "analyze" => CommReport::XBoard(XBoardReport::Analyze),
            cmd if cmd == "." => CommReport::XBoard(XBoardReport::Dot),
            cmd if cmd == "exit" => CommReport::XBoard(XBoardReport::Exit),
            cmd if cmd == "quit" => CommReport::XBoard(XBoardReport::Quit),

            // Commands the engine is going to ignore; either because no
            // response is required, or the functionality is not (yet)
            // implemented.
            cmd if cmd == "random" => CommReport::XBoard(XBoardReport::Random),
            cmd if cmd == "easy" => CommReport::XBoard(XBoardReport::Easy),
            cmd if cmd == "hard" => CommReport::XBoard(XBoardReport::Hard),

            // Custom commands
            cmd if cmd == "board" => CommReport::XBoard(XBoardReport::Board),
            cmd if cmd == "history" => CommReport::XBoard(XBoardReport::History),
            cmd if cmd == "legal" => CommReport::XBoard(XBoardReport::Legal),
            cmd if cmd == "eval" => CommReport::XBoard(XBoardReport::Eval),
            cmd if cmd == "help" => CommReport::XBoard(XBoardReport::Help),

            // Everything else is ignored.
            _ => CommReport::XBoard(XBoardReport::Unknown(i)),
        }
    }
}

// Parse incoming XBoard commands from the GUI.
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

// Responses to incoming XBoard commands. These are sent to the GUI.
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

    fn search_summary(s: SearchSummary) {
        // (Send time in 1/100th instead of 1/1000th.)
        println!(
            "{} {} {} {} {}",
            s.depth,
            s.cp,
            (s.time as f64 / 10.0).round(),
            s.nodes,
            s.pv_as_string()
        );
    }

    fn search_stat01(s: XBoardStat01) {
        // (Report time in 1/100th instead of 1/1000th.)
        let stats = format!(
            "{} {} {} {} {} {}",
            (s.time as f64 / 10.0).round(),
            s.nodes,
            s.depth,
            s.moves_left,
            s.total_moves,
            s.curr_move.as_string()
        );

        println!("stat01: {}", stats);
    }
}
