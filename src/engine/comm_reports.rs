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

use super::{
    defs::{ErrFatal, ErrNormal, Quiet, XBoardStat01},
    Engine,
};
use crate::{
    comm::{uci::UciReport, xboard::XBoardReport, CommControl, CommReport},
    defs::{About, FEN_START_POSITION},
    engine::defs::EngineOptionName,
    evaluation::evaluate_position,
    search::defs::{SearchControl, SearchMode, SearchParams, OVERHEAD},
};

// This block implements handling of incoming information, which will be in
// the form of either Comm or Search reports.
impl Engine {
    pub fn comm_reports(&mut self, comm_report: &CommReport) {
        // Split out the comm reports according to their source.
        match comm_report {
            CommReport::Uci(u) => self.cr_uci(u),
            CommReport::XBoard(x) => self.cr_xboard(x),
        }
    }
}

// Handles "Uci" Comm reports sent by the UCI module.
impl Engine {
    fn cr_uci(&mut self, uci_report: &UciReport) {
        // Search parameters to send into the search thread.
        let mut sp = SearchParams::new();

        match uci_report {
            // UCI commands.
            UciReport::Uci => self.comm.send(CommControl::Identify),

            UciReport::IsReady => self.comm.send(CommControl::Ready),

            UciReport::UciNewGame => {
                self.board
                    .lock()
                    .expect(ErrFatal::LOCK)
                    .fen_read(Some(FEN_START_POSITION))
                    .expect(ErrFatal::NEW_GAME);
                self.tt_search.lock().expect(ErrFatal::LOCK).clear();
            }

            UciReport::SetOption(option) => {
                match option {
                    EngineOptionName::Hash(value) => {
                        if let Ok(v) = value.parse::<usize>() {
                            self.tt_search.lock().expect(ErrFatal::LOCK).resize(v);
                        } else {
                            let msg = String::from(ErrNormal::NOT_INT);
                            self.comm.send(CommControl::InfoString(msg));
                        }
                    }

                    EngineOptionName::ClearHash => {
                        self.tt_search.lock().expect(ErrFatal::LOCK).clear()
                    }

                    EngineOptionName::Nothing => (),
                };
            }

            UciReport::Position(fen, moves) => {
                let fen_result = self.board.lock().expect(ErrFatal::LOCK).fen_read(Some(fen));

                if fen_result.is_ok() {
                    self.create_legal_move_list();
                    for m in moves.iter() {
                        let ok = self.execute_move(m.clone());
                        if !ok {
                            let msg = format!("{}: {}", m, ErrNormal::NOT_LEGAL);
                            self.comm.send(CommControl::InfoString(msg));
                            break;
                        }
                    }
                }

                if fen_result.is_err() {
                    let msg = ErrNormal::FEN_FAILED.to_string();
                    self.comm.send(CommControl::InfoString(msg));
                }
            }

            UciReport::GoInfinite => {
                sp.search_mode = SearchMode::Infinite;
                self.search.send(SearchControl::Start(sp));
            }

            UciReport::GoDepth(depth) => {
                sp.depth = *depth;
                sp.search_mode = SearchMode::Depth;
                self.search.send(SearchControl::Start(sp));
            }

            UciReport::GoMoveTime(msecs) => {
                sp.move_time = *msecs - (OVERHEAD as u128);
                sp.search_mode = SearchMode::MoveTime;
                self.search.send(SearchControl::Start(sp));
            }

            UciReport::GoNodes(nodes) => {
                sp.nodes = *nodes;
                sp.search_mode = SearchMode::Nodes;
                self.search.send(SearchControl::Start(sp));
            }

            UciReport::GoGameTime(gt) => {
                sp.game_time = *gt;
                sp.search_mode = SearchMode::GameTime;
                self.search.send(SearchControl::Start(sp));
            }

            UciReport::Stop => self.search.send(SearchControl::Stop),

            UciReport::Quit => self.quit(),

            // Custom commands
            UciReport::Board => self.comm.send(CommControl::PrintBoard),
            UciReport::History => self.comm.send(CommControl::PrintHistory),
            UciReport::Legal => {
                let ml = Box::new(self.legal_moves);
                self.comm.send(CommControl::PrintLegal(ml))
            }
            UciReport::Eval => {
                let evaluation = evaluate_position(&self.board.lock().expect(ErrFatal::LOCK));
                self.comm.send(CommControl::PrintEval(evaluation));
            }
            UciReport::Help => self.comm.send(CommControl::PrintHelp),
            UciReport::Unknown => (),
        }
    }
}

impl Engine {
    // Handles "XBoard" Comm reports send by the XBoard module.
    fn cr_xboard(&mut self, xboard_report: &XBoardReport) {
        // Search parameters to send into the search thread.
        let mut sp = SearchParams::new();

        match xboard_report {
            // Send an empty response (print a new line) to make sure the
            // output buffer is flushed.
            XBoardReport::XBoard => self.comm.send(CommControl::Empty),

            // XBoard commands "protover X" is similar to command "uci".
            // The engine replies with an identification and a list of
            // features and options it supports.
            XBoardReport::ProtoVer(v) => {
                if *v == 2 {
                    self.comm.send(CommControl::Identify);
                    self.comm.send(CommControl::Ready);
                } else {
                    let msg = format!("# {} only supports XBoard version 2.", About::ENGINE);
                    self.comm.send(CommControl::Message(msg));
                }
            }

            // Engine is alive: reply to incoming "ping n" with "pong n"
            XBoardReport::Ping(n) => self.comm.send(CommControl::Pong(*n)),

            // Set each feature accepted by the GUI to true; in case we
            // need to know this later, somewhere in the engine.
            XBoardReport::Accepted(feature) => match &feature[..] {
                f if f == "done" => self.xboard.features.done = true,
                f if f == "ping" => self.xboard.features.ping = true,
                f if f == "setboard" => self.xboard.features.setboard = true,
                f if f == "usermove" => self.xboard.features.usermove = true,
                f if f == "debug" => self.xboard.features.debug = true,
                f if f == "sigint" => self.xboard.features.sigint = true,
                f if f == "sigterm" => self.xboard.features.sigterm = true,
                _ => (),
            },

            // xboard "setboard <fen>" is equivalent to uci "position
            // <fen>", but without the "moves" part.
            XBoardReport::SetBoard(fen) => {
                let fen_result = self.board.lock().expect(ErrFatal::LOCK).fen_read(Some(fen));

                if fen_result.is_ok() {
                    self.create_legal_move_list();
                } else {
                    let msg = format!("# {}", ErrNormal::FEN_FAILED.to_string());
                    self.comm.send(CommControl::Message(msg));
                }
            }

            XBoardReport::New => println!("New command received."),

            XBoardReport::Question => println!("? received."),

            // Either do or don't post (print) analysis results
            XBoardReport::Post => self.settings.quiet = Quiet::No,
            XBoardReport::NoPost => self.settings.quiet = Quiet::Silent,

            // xboard "analyze" is equivalent to uci "go infinite"
            XBoardReport::Analyze => {
                sp.search_mode = SearchMode::Infinite;
                self.search.send(SearchControl::Start(sp));
            }

            XBoardReport::Dot => {
                if self.xboard.stat01.is_complete() {
                    let s = self.xboard.stat01;
                    self.comm.send(CommControl::AnalyzeStat01(s));
                }
            }

            // xboard "exit" is roughly equivalent to uci "stop"
            XBoardReport::Exit => {
                self.xboard.stat01 = XBoardStat01::new();
                self.search.send(SearchControl::Stop);
            }

            XBoardReport::Quit => self.quit(),

            // Ignore the following incoming reports from the XBoard comm
            XBoardReport::Random => (), // Engine doesn't support move randomization.
            XBoardReport::Easy => (),   // Pondering off (Pondering not yet supported.)
            XBoardReport::Hard => (),   // Pondering on (Pondering not yet supported.)

            // Custom commands
            XBoardReport::Board => self.comm.send(CommControl::PrintBoard),
            XBoardReport::History => self.comm.send(CommControl::PrintHistory),
            XBoardReport::Legal => {
                let ml = Box::new(self.legal_moves);
                self.comm.send(CommControl::PrintLegal(ml));
            }
            XBoardReport::Eval => {
                let evaluation = evaluate_position(&self.board.lock().expect(ErrFatal::LOCK));
                self.comm.send(CommControl::PrintEval(evaluation));
            }
            XBoardReport::Help => self.comm.send(CommControl::PrintHelp),
            XBoardReport::Unknown(cmd) => {
                let msg = format!("# Command '{}' is unknown or not implemented.", cmd);
                self.comm.send(CommControl::Message(msg));
            }
        }
    }
}
