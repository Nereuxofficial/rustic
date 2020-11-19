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

use super::{
    defs::{ErrFatal, ErrNormal},
    Engine,
};
use crate::{
    comm::{uci::UciReport, xboard::XBoardReport, CommControl, CommReport},
    defs::{About, FEN_START_POSITION},
    evaluation::evaluate_position,
    search::defs::{SearchControl, SearchMode, SearchParams},
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

    // Handles "Uci" Comm reports sent by the UCI-module.
    fn cr_uci(&mut self, uci_report: &UciReport) {
        // Setup default variables.
        let mut sp = SearchParams::new();
        sp.quiet = self.settings.quiet;

        match uci_report {
            // UCI commands.
            UciReport::Uci => self.comm.send(CommControl::Identify),

            UciReport::UciNewGame => self
                .board
                .lock()
                .expect(ErrFatal::LOCK)
                .fen_read(Some(FEN_START_POSITION))
                .expect(ErrFatal::NEW_GAME),

            UciReport::IsReady => self.comm.send(CommControl::Ready),

            UciReport::Position(fen, moves) => {
                let fen_result = self.board.lock().expect(ErrFatal::LOCK).fen_read(Some(fen));

                if fen_result.is_ok() {
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
                sp.move_time = *msecs;
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

            UciReport::Eval => {
                let evaluation = evaluate_position(&self.board.lock().expect(ErrFatal::LOCK));
                let msg = format!("{} centipawns", evaluation);
                self.comm.send(CommControl::Message(msg));
            }

            UciReport::Help => self.comm.send(CommControl::PrintHelp),

            UciReport::Unknown => (),
        }
    }

    fn cr_xboard(&mut self, xboard_report: &XBoardReport) {
        match xboard_report {
            // XBoard commands
            XBoardReport::XBoard => (), // No action required.

            XBoardReport::ProtoVer(v) => {
                if *v == 2 {
                    self.comm.send(CommControl::Identify);
                    self.comm.send(CommControl::Ready);
                } else {
                    let msg = format!("# {} only supports XBoard version 2.", About::ENGINE);
                    self.comm.send(CommControl::Message(msg));
                }
            }

            XBoardReport::Ping(n) => self.comm.send(CommControl::Pong(*n)),

            // Set each feature setting accepted by the GUI to true.
            XBoardReport::Accepted(feature) => match &feature[..] {
                f if f == "done" => self.settings.xbfeatures.done = true,
                f if f == "ping" => self.settings.xbfeatures.ping = true,
                f if f == "setboard" => self.settings.xbfeatures.setboard = true,
                f if f == "usermove" => self.settings.xbfeatures.usermove = true,
                f if f == "debug" => self.settings.xbfeatures.debug = true,
                f if f == "sigint" => self.settings.xbfeatures.sigint = true,
                f if f == "sigterm" => self.settings.xbfeatures.sigterm = true,
                _ => (),
            },

            XBoardReport::Quit => self.quit(),

            // Custom commands
            XBoardReport::Board => self.comm.send(CommControl::PrintBoard),
            XBoardReport::History => self.comm.send(CommControl::PrintHistory),
            XBoardReport::Eval => {
                let evaluation = evaluate_position(&self.board.lock().expect(ErrFatal::LOCK));
                let msg = format!("# {} centipawns", evaluation);
                self.comm.send(CommControl::Message(msg));
            }
            XBoardReport::Help => self.comm.send(CommControl::PrintHelp),
            XBoardReport::Unknown(cmd) => {
                let msg = format!("# Command '{}' is unknown or not implemented.", cmd);
                self.comm.send(CommControl::Message(msg));
            }
        }
    }
}
