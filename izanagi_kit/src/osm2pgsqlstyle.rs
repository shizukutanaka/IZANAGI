//! osm2pgsql Lua スタイルファイル (`default.style` / `.lua` スタイル) の
//! 検出・カウント。
//!
//! `osm2pgsql.define_*_table` / `tables.*` 代入・`key =`/`type =` カラム定義・
//! `keys =` タグ選択を持つ Lua スタイル定義。
//!
//! ```
//! let lua = b"local tables = {}\ntables.polygons = osm2pgsql.define_table({name='poly',keys={'a'}})\n";
//! assert!(izanagi_kit::osm2pgsqlstyle::detect(lua));
//! let c = izanagi_kit::osm2pgsqlstyle::parse(lua).unwrap();
//! assert_eq!(c.tables, 1);
//! ```

/// カウント結果。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Counts {
    /// 非空・非コメント行の総数。
    pub entries: usize,
    /// `osm2pgsql.define_*`/`osm2pgsql.define_table` 定義呼出数。
    pub tables: usize,
    /// `{ key = 'x', type = 'y' }` カラム定義の `key`/`type`/`not_null`/`create_index` 出現数。
    pub columns: usize,
    /// `keys =`/`relation_name`/`members`/`name` タグ選択式の出現数。
    pub tags: usize,
    /// `local`/`function`/`if`/`for`/`return`/`end` 等 Lua 構文行数。
    pub lua: usize,
    /// `tables.*` への代入行数。
    pub assigns: usize,
    /// その他行数。
    pub misc: usize,
}

/// `b` が osm2pgsql スタイルかどうか。
#[must_use]
pub fn detect(b: &[u8]) -> bool {
    parse(b).is_some_and(|c| c.tables + c.assigns >= 1)
}

/// `b` を osm2pgsql スタイルとして解析する。
#[must_use]
pub fn parse(b: &[u8]) -> Option<Counts> {
    let text = std::str::from_utf8(b).ok()?;
    if !text.contains("osm2pgsql.") {
        return None;
    }
    let mut c = Counts {
        entries: 0,
        tables: 0,
        columns: 0,
        tags: 0,
        lua: 0,
        assigns: 0,
        misc: 0,
    };
    for line in text.lines() {
        let t = line.trim();
        if t.is_empty() || t.starts_with("--") {
            continue;
        }
        c.entries += 1;
        let mut hit = false;
        if t.contains("osm2pgsql.define_") {
            c.tables += 1;
            hit = true;
        }
        if t.contains("key =")
            || t.contains("type =")
            || t.contains("not_null")
            || t.contains("create_index")
        {
            c.columns += 1;
            hit = true;
        }
        if t.contains("keys =") || t.contains("relation_name") || t.contains("members") {
            c.tags += 1;
            hit = true;
        }
        if t.starts_with("local")
            || t.starts_with("function")
            || t.starts_with("if ")
            || t.starts_with("for ")
            || t.starts_with("return")
            || t.starts_with("end")
        {
            c.lua += 1;
            hit = true;
        }
        if t.starts_with("tables.") {
            c.assigns += 1;
            hit = true;
        }
        if !hit {
            c.misc += 1;
        }
    }
    (c.tables >= 1).then_some(c)
}

#[cfg(test)]
mod tests {
    use super::parse;

    #[test]
    fn style() {
        let lua = b"-- style\nlocal tables = {}\ntables.polygons = osm2pgsql.define_table({\n name = 'poly',\n keys = { 'a', 'b' },\n columns = {\n  { key = 'name', type = 'text' },\n  { key = 'geom', type = 'geometry', not_null = true },\n }})\ntables.nodes = osm2pgsql.define_node_table('n')\n";
        let c = parse(lua).unwrap();
        assert_eq!(c.tables, 2);
        assert_eq!(c.assigns, 2);
        assert!(c.columns >= 2);
        assert!(c.tags >= 1);
        assert_eq!(c.lua, 1);
    }

    #[test]
    fn not_style() {
        assert!(parse(b"local x = 1\nprint(x)\n").is_none());
    }
}
