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

use super::Engine;
use crate::defs::About;

// This notice is displayed if the engine is a debug binary. (Debug
// binaries are unoptimized and slower than release binaries.)
#[cfg(debug_assertions)]
const NOTICE_DEBUG_MODE: &str = "Notice: Running in debug mode";

impl Engine {
    pub fn print_ascii_logo(&self) {
        println!();
        println!("d888888b                      dP   oo        ");
        println!("88     88                     88             ");
        println!("88oooo88  88    88  d8888b  d8888P dP d88888b");
        println!("88    88  88    88  8ooooo    88   88 88     ");
        println!("88     88 88    88       88   88   88 88     ");
        println!("88     88  88888P  888888P    dP   dP 888888P");
        println!("ooooooooooooooooooooooooooooooooooooooooooooo");
        println!();
    }

    // Print information about the engine.
    pub fn print_about(&self) {
        println!("Engine: {} {}", About::ENGINE, About::VERSION);
        println!("Author: {}", About::AUTHOR);
        println!("EMail: {}", About::EMAIL);
        println!("Website: {}", About::WEBSITE);
    }

    pub fn print_short_about(&self, threads: usize, protocol: &str) {
        let t = if threads == 1 { "thread" } else { "threads" };
        println!(
            "{} {} by {} ({} mode, {} {})",
            About::ENGINE,
            About::VERSION,
            About::AUTHOR,
            protocol,
            threads,
            t
        );
    }

    pub fn print_settings(&self, threads: usize, protocol: &str) {
        println!("Protocol: {}", protocol);
        println!("Threads: {}", threads);
        #[cfg(debug_assertions)]
        println!("{}", NOTICE_DEBUG_MODE);
    }
}
