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

mod about;
mod comm_reports;
pub mod defs;
mod main_loop;
mod search_reports;
mod utils;

use crate::{
    board::Board,
    comm::{uci::Uci, xboard::XBoard, CommControl, CommType, IComm},
    defs::EngineRunResult,
    engine::defs::{ErrFatal, Information, Settings, XBoardFeatures, XBoardSpecifics},
    misc::{cmdline::CmdLine, perft},
    movegen::{defs::MoveList, MoveGenerator},
    search::{defs::SearchControl, Search},
};
use crossbeam_channel::Receiver;
use std::sync::{Arc, Mutex};

#[cfg(feature = "extra")]
use crate::{
    board::defs::Pieces,
    extra::{testsuite, wizardry},
};

// This struct holds the chess engine and its functions, so they are not
// all seperate entities in the global space.
pub struct Engine {
    quit: bool,                             // Flag that will quit the main thread.
    settings: Settings,                     // Struct holding all the settings.
    xboard: XBoardSpecifics,                // Storage for XBoard specifics.
    cmdline: CmdLine,                       // Command line interpreter.
    comm: Box<dyn IComm>,                   // Communications (active).
    board: Arc<Mutex<Board>>,               // This is the main engine board.
    mg: Arc<MoveGenerator>,                 // Move Generator.
    info_rx: Option<Receiver<Information>>, // Receiver for incoming information.
    search: Search,                         // Search object (active).
    legal_moves: MoveList,                  // Legal moves in current position
}

impl Engine {
    // Create e new engine.
    pub fn new() -> Self {
        // Create the command-line object.
        let cmdline = CmdLine::new();

        // Create the communication interface
        let comm: Box<dyn IComm> = match &cmdline.comm()[..] {
            CommType::XBOARD => Box::new(XBoard::new()),
            CommType::UCI => Box::new(Uci::new()),
            _ => panic!(ErrFatal::CREATE_COMM),
        };

        // Get engine settings from the command-line
        let threads = cmdline.threads();
        let quiet = cmdline.quiet();

        // Create the engine itself.
        Self {
            quit: false,
            settings: Settings { threads, quiet },
            xboard: XBoardSpecifics {
                features: XBoardFeatures {
                    done: false,
                    ping: false,
                    setboard: false,
                    usermove: false,
                    debug: false,
                    sigint: false,
                    sigterm: false,
                },
            },
            cmdline,
            comm,
            board: Arc::new(Mutex::new(Board::new())),
            mg: Arc::new(MoveGenerator::new()),
            info_rx: None,
            search: Search::new(),
            legal_moves: MoveList::new(),
        }
    }

    // Run the engine.
    pub fn run(&mut self) -> EngineRunResult {
        let protocol = self.comm.get_protocol_name();
        if protocol != CommType::XBOARD {
            self.print_ascii_logo();
            self.print_about();
            self.print_settings(self.settings.threads, protocol);
            println!();
        } else {
            self.print_short_about(self.settings.threads, protocol);
        }

        // Setup position and abort if this fails.
        self.setup_position()?;
        self.create_legal_move_list();

        // Run a specific action if requested...
        let mut action_requested = false;

        // Run perft if requested.
        if self.cmdline.perft() > 0 {
            action_requested = true;
            perft::run(self.board.clone(), self.cmdline.perft(), self.mg.clone());
        }

        // === Only available with "extra" features enabled. ===
        #[cfg(feature = "extra")]
        // Generate magic numbers if requested.
        if self.cmdline.has_wizardry() {
            action_requested = true;
            wizardry::find_magics(Pieces::ROOK);
            wizardry::find_magics(Pieces::BISHOP);
        };

        #[cfg(feature = "extra")]
        // Run large EPD test suite if requested.
        if self.cmdline.has_test() {
            action_requested = true;
            testsuite::run();
        }
        // =====================================================

        // In the main loop, the engine manages its resources so it will be
        // able to play legal chess and communicate with different user
        // interfaces.
        if !action_requested {
            self.main_loop();
        }

        // There are three ways to exit the engine: when the FEN-setup
        // fails, because of a crash, or normally. In the first two cases,
        // this Ok(()) won't be reached.
        Ok(())
    }

    // This function quits Commm, Search, and then the engine thread itself.
    pub fn quit(&mut self) {
        self.search.send(SearchControl::Quit);
        self.comm.send(CommControl::Quit);
        self.quit = true;
    }
}
