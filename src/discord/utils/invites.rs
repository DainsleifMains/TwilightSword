// © 2024-2025 ElementalAlchemist and the Dainsleif Mains Development Team
//
// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

/// Turns a Discord invite URL into an invite code
pub fn invite_code_from_url(url: &str) -> Option<String> {
	match url
		.strip_prefix("https://discord.gg/")
		.or_else(|| url.strip_prefix("https://discord.com/invite/"))
	{
		Some(code) => {
			let code = match code.split_once('/') {
				Some((code, _)) => code,
				None => code,
			};
			let code = match code.split_once('?') {
				Some((code, _)) => code,
				None => code,
			};
			Some(code.to_string())
		}
		None => None,
	}
}
