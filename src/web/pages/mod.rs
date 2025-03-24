// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

pub mod app;
mod errors;
mod guild;
mod header;
#[cfg(feature = "ssr")]
mod server_utils;
#[cfg(feature = "ssr")]
pub mod shell;
mod staff;
mod utils;
