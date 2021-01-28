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

use super::{defs::Quiet, Engine};
use crate::{
    comm::{CommControl, CommType},
    search::defs::SearchReport,
};

impl Engine {
    pub fn search_reports(&mut self, search_report: &SearchReport) {
        match search_report {
            SearchReport::Finished(m) => {
                self.comm.send(CommControl::BestMove(*m));
            }

            SearchReport::SearchSummary(summary) => {
                let xboard = self.comm.get_protocol_name() == CommType::XBOARD;
                let silent = self.settings.quiet == Quiet::Silent;

                // If in XBoard mode, start filling the stat01 var. These
                // can be requested by the GUI by sending the "." command.
                if xboard {
                    self.xboard.stat01.time = summary.time;
                    self.xboard.stat01.nodes = summary.nodes;
                    self.xboard.stat01.depth = summary.depth;
                }

                if !silent {
                    self.comm.send(CommControl::SearchSummary(summary.clone()));
                }
            }

            SearchReport::SearchCurrentMove(cm) => {
                let xboard = self.comm.get_protocol_name() == CommType::XBOARD;

                // If in XBoard mode, update xboard's search stats.
                if xboard {
                    let total_moves = self.legal_moves.len();
                    let move_number = cm.curr_move_number;

                    self.xboard.stat01.moves_left = total_moves - move_number;
                    self.xboard.stat01.total_moves = total_moves;
                    self.xboard.stat01.curr_move = cm.curr_move;
                }

                // Send current move if not in xboard mode and not quiet.
                if !xboard && (self.settings.quiet == Quiet::No) {
                    self.comm.send(CommControl::SearchCurrMove(*cm));
                }
            }

            SearchReport::SearchStats(stats) => {
                let xboard = self.comm.get_protocol_name() == CommType::XBOARD;

                // If in XBoard mode, update xboard's search stats.
                if xboard {
                    self.xboard.stat01.time = stats.time;
                    self.xboard.stat01.nodes = stats.nodes;
                }

                // Send stats if not in xboard mode and not quiet.
                if !xboard && (self.settings.quiet == Quiet::No) {
                    self.comm.send(CommControl::SearchStats(*stats));
                }
            }
        }
    }
}
