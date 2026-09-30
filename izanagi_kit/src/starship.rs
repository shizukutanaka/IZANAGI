//! Starship `starship.toml` census.
//!
//! Top-level keys: `format`, `right_format`,
//! `continuation_prompt`, `add_newline`, `scan_timeout`,
//! `command_timeout`, `follow_symlinks`, `palette`,
//! `palettes.<name>`, `augment_strategy`.
//! Module tables: `[username]`/`[hostname]`/`[localip]`/
//! `[directory]`/`[git_branch]`/`[git_status]`/`[git_state]`/
//! `[git_metrics]`/`[git_commit]`/`[hg_branch]`/`[pijul_channel]`/
//! `[fossil_branch]`/`[fossil_metrics]`/`[character]`/
//! `[cmd_duration]`/`[jobs]`/`[status]`/`[sudo]`/`[shell]`/
//! `[shlvl]`/`[line_break]`/`[fill]`/`[battery]`/`[time]`/
//! `[memory_usage]`/`[swap]`/`[os]`/`[env_var]`/`[env_var.NAME]`/
//! `[custom]`/`[custom.NAME]` + language modules
//! `[nodejs]`/`[rust]`/`[python]`/`[golang]`/`[java]`/`[kotlin]`/
//! `[scala]`/`[swift]`/`[zig]`/`[c]`/`[cpp]`/`[lua]`/`[ruby]`/
//! `[php]`/`[perl]`/`[dart]`/`[deno]`/`[dotnet]`/`[elixir]`/
//! `[elm]`/`[erlang]`/`[haskell]`/`[julia]`/`[nim]`/`[ocaml]`/
//! `[rlang]`/`[crystal]`/`[buf]`/`[cobol]`/`[daml]`/`[fennel]`/
//! `[fortran]`/`[haxe]`/`[helm]`/`[meson]`/`[mojo]`/`[opa]`/
//! `[quarto]`/`[raku]`/`[red]`/`[solidity]`/`[typst]`/`[v]`/
//! `[vagrant]` + cloud/tool modules `[aws]`/`[azure]`/`[gcloud]`/
//! `[openstack]`/`[container]`/`[docker_context]`/`[kubernetes]`/
//! `[terraform]`/`[pulumi]`/`[nats]`/`[vcsh]`/`[netns]`/`[singularity]`/
//! `[spack]`/`[systemd]`/`[nix_shell]`/`[guix_shell]`/`[conda]`/
//! `[pixi]`/`[package]`/`[cmake]`/`[vlang]`.
//!
//! ```rust
//! let s = "format = \"$all\"\nadd_newline = false\n[directory]\nstyle = \"blue\"\n[git_branch]\nformat = \"[$symbol$branch]($style)\"\n";
//! let c = izanagi_kit::starship::Starship::parse(s.as_bytes()).unwrap();
//! assert_eq!(c.sections, 2);
//! ```

/// starship.toml census.
#[derive(Debug, Clone)]
pub struct Starship {
    /// `[…]`/`[palettes.*]`/`[custom.*]` tables.
    pub sections: usize,
    /// `key = value` pairs.
    pub settings: usize,
    /// `#` comment lines.
    pub comments: usize,
}

/// Detect starship.toml content.
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    let t = match std::str::from_utf8(b) {
        Ok(t) => t,
        Err(_) => return false,
    };
    let mut hits = 0usize;
    for p in [
        "[git_branch]",
        "[directory]",
        "[character]",
        "[cmd_duration]",
        "format = ",
        "continuation_prompt",
        "scan_timeout",
        "command_timeout",
        "[custom.",
        "[palettes",
        "[env_var",
        "$all",
    ] {
        if t.contains(p) {
            hits += 1;
        }
    }
    hits >= 2
}

impl Starship {
    /// Census a starship.toml buffer.
    #[must_use]
    pub fn parse(b: &[u8]) -> Option<Self> {
        if !detect(b) {
            return None;
        }
        let t = std::str::from_utf8(b).ok()?;
        let mut c = Self {
            sections: 0,
            settings: 0,
            comments: 0,
        };
        for line in t.lines() {
            let s = line.trim();
            if s.is_empty() {
                continue;
            }
            if s.starts_with('#') {
                c.comments += 1;
                continue;
            }
            if s.starts_with('[') {
                c.sections += 1;
                continue;
            }
            if let Some(eq) = s.find('=') {
                let key = s[..eq].trim();
                if !key.is_empty() {
                    c.settings += 1;
                }
            }
        }
        Some(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_starship() {
        let b = b"format = \"$all\"\n[directory]\n";
        assert!(detect(b));
    }

    #[test]
    fn rejects_other() {
        assert!(!detect(b"[x]\ny=1\n"));
        assert!(!detect(b""));
    }

    #[test]
    fn parses_starship() {
        let b = concat!(
            "# starship\n",
            "format = \"$all$directory$character\"\n",
            "add_newline = false\n",
            "scan_timeout = 30\n",
            "command_timeout = 500\n",
            "[palette]\n",
            "[palettes.catppuccin]\n",
            "text = \"#cdd6f4\"\n",
            "[username]\n",
            "show_always = true\n",
            "format = \"[$user]($style)\"\n",
            "style_user = \"bold yellow\"\n",
            "[hostname]\n",
            "ssh_only = true\n",
            "[directory]\n",
            "truncation_length = 3\n",
            "truncate_to_repo = true\n",
            "style = \"bold blue\"\n",
            "read_only = \" ro\"\n",
            "[git_branch]\n",
            "format = \"[$symbol$branch]($style)\"\n",
            "symbol = \" \"\n",
            "[git_status]\n",
            "conflicted = \"=\"\n",
            "ahead = \"⇡${count}\"\n",
            "behind = \"⇣${count}\"\n",
            "[git_metrics]\n",
            "added_style = \"bold green\"\n",
            "[character]\n",
            "success_symbol = \"[➜](bold green)\"\n",
            "error_symbol = \"[➜](bold red)\"\n",
            "vimcmd_symbol = \"[❮](bold green)\"\n",
            "[cmd_duration]\n",
            "min_time = 500\n",
            "format = \"[$duration]($style)\"\n",
            "[nodejs]\n",
            "format = \"[$symbol$version]($style)\"\n",
            "[rust]\n",
            "format = \"[$symbol$version]($style)\"\n",
            "[kubernetes]\n",
            "disabled = true\n",
            "[env_var.FOO]\n",
            "default = \"bar\"\n",
            "[custom.jj]\n",
            "command = \"jj log\"\n",
            "when = true\n",
            "[battery]\n",
            "full_symbol = \"🔋\"\n",
            "[time]\n",
            "disabled = false\n",
            "time_format = \"%R\"\n",
        );
        let c = Starship::parse(b.as_bytes()).unwrap();
        assert_eq!(c.sections, 17);
        assert!(c.settings >= 20);
        assert_eq!(c.comments, 1);
    }
}
