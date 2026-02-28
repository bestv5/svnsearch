//! 搜索查询解析与匹配：操作符 AND/OR/NOT、引号短语、通配符 *?、修饰符 ascii/case/diacritics/file/folder。
//! 默认仅对「名称」（文件名/目录名）匹配，不匹配完整 path。

use regex::Regex;
use std::ops::Range;

/// 匹配配置（由修饰符解析得到）
#[derive(Debug, Clone, Default)]
pub struct MatchConfig {
    /// 区分大小写（默认 false）
    pub case_sensitive: bool,
    /// 仅对 ASCII 做大小写折叠（默认 false = 全 Unicode）
    pub ascii_fold_only: bool,
    /// 区分变音符（默认 false = 忽略变音符）
    pub diacritics_sensitive: bool,
    /// 类型过滤
    pub type_filter: TypeFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TypeFilter {
    #[default]
    Both,
    FileOnly,
    FolderOnly,
}

/// AST：优先级 ! > AND > OR
#[derive(Debug, Clone)]
pub enum Expr {
    Or(Vec<Expr>),
    And(Vec<Expr>),
    Not(Box<Expr>),
    Term(String),
    Phrase(String),
}

/// 解析结果：配置 + 表达式
pub struct ParsedQuery {
    pub config: MatchConfig,
    pub expr: Option<Expr>,
}

/// 高亮区间（字节偏移，左闭右开）
pub type HighlightRange = Range<usize>;

/// 对字符串做规范化以便比较（大小写；变音符暂不折叠，diacritics 仅控制是否精确匹配），返回规范化串及「规范化每字符 → 原串字节区间」映射
fn fold_for_match(s: &str, config: &MatchConfig) -> (String, Vec<(usize, usize)>) {
    let mut out = String::new();
    let mut map: Vec<(usize, usize)> = Vec::new();
    for (byte_start, ch) in s.char_indices() {
        let byte_end = byte_start + ch.len_utf8();
        map.push((byte_start, byte_end));
        let folded = fold_char(&ch.to_string(), config);
        out.push_str(&folded);
    }
    (out, map)
}

fn fold_char(s: &str, config: &MatchConfig) -> String {
    if config.case_sensitive {
        return s.to_string();
    }
    if config.ascii_fold_only {
        return s
            .chars()
            .map(|c| {
                if c.is_ascii_alphabetic() {
                    c.to_ascii_lowercase()
                } else {
                    c
                }
            })
            .collect();
    }
    s.to_lowercase()
}

/// 将 glob 模式（* ?）转为正则，并转义其他正则元字符
fn glob_to_regex(glob: &str) -> Result<Regex, String> {
    let mut re = String::new();
    re.push('^');
    for c in glob.chars() {
        match c {
            '*' => re.push_str(".*"),
            '?' => re.push('.'),
            '.' | '+' | '(' | ')' | '[' | ']' | '{' | '}' | '^' | '$' | '|' | '\\' => {
                re.push('\\');
                re.push(c);
            }
            _ => re.push(c),
        }
    }
    re.push('$');
    Regex::new(&re).map_err(|e| format!("无效通配符模式: {}", e))
}

/// 检查 term 是否包含通配符
fn has_glob(s: &str) -> bool {
    s.contains('*') || s.contains('?')
}

/// 在规范化后的文本中找 pattern（已规范化）的子串，返回原串中的字节区间
/// 注意：find() 返回的是字节偏移，map 按字符索引，需用 byte_offset_to_char_index 转换
fn find_substring_ranges(
    orig: &str,
    folded_orig: &str,
    folded_pattern: &str,
    map: &[(usize, usize)],
) -> Vec<HighlightRange> {
    if folded_pattern.is_empty() || map.is_empty() {
        return vec![];
    }
    let mut ranges = Vec::new();
    let mut start_byte = 0;
    while let Some(rel_byte) = folded_orig[start_byte..].find(folded_pattern) {
        let abs_start_byte = start_byte + rel_byte;
        let abs_end_byte = abs_start_byte + folded_pattern.len();
        let start_char = byte_offset_to_char_index(folded_orig, abs_start_byte);
        let end_char = byte_offset_to_char_index(folded_orig, abs_end_byte);
        let byte_start = if start_char < map.len() {
            map[start_char].0
        } else {
            orig.len()
        };
        let byte_end = if end_char > 0 && end_char <= map.len() {
            map[end_char - 1].1
        } else {
            orig.len()
        };
        ranges.push(byte_start..byte_end);
        start_byte = abs_start_byte + 1;
    }
    ranges
}

fn byte_offset_to_char_index(s: &str, byte_pos: usize) -> usize {
    for (char_idx, (byte_off, ch)) in s.char_indices().enumerate() {
        if byte_pos < byte_off + ch.len_utf8() {
            return char_idx;
        }
    }
    s.chars().count()
}

/// 用正则找匹配区间（返回原串字节偏移）。正则匹配在 folded 串上，match 的 start/end 为字节偏移
fn find_regex_ranges(
    orig: &str,
    folded_orig: &str,
    re: &Regex,
    map: &[(usize, usize)],
) -> Vec<HighlightRange> {
    if map.is_empty() {
        return vec![];
    }
    let mut ranges = Vec::new();
    for cap in re.captures_iter(folded_orig) {
        let m = cap.get(0).unwrap();
        let start_char = byte_offset_to_char_index(folded_orig, m.start());
        let end_char = byte_offset_to_char_index(folded_orig, m.end());
        let byte_start = if start_char < map.len() {
            map[start_char].0
        } else {
            orig.len()
        };
        let byte_end = if end_char > 0 && end_char <= map.len() {
            map[end_char - 1].1
        } else {
            orig.len()
        };
        ranges.push(byte_start..byte_end);
    }
    ranges
}

/// 将名称与高亮区间切分为前端可渲染的分段 [(text, highlight), ...]
pub fn ranges_to_segments(name: &str, ranges: &[HighlightRange]) -> Vec<(String, bool)> {
    let mut sorted: Vec<HighlightRange> = ranges.to_vec();
    sorted.sort_by_key(|r| r.start);
    let merged = merge_ranges(sorted);
    let mut out = Vec::new();
    let mut pos = 0;
    for r in merged {
        if r.start > pos && r.start <= name.len() {
            out.push((name[pos..r.start].to_string(), false));
        }
        if r.end <= name.len() && r.start < r.end {
            out.push((name[r.start..r.end].to_string(), true));
        }
        pos = r.end.min(name.len());
    }
    if pos < name.len() {
        out.push((name[pos..].to_string(), false));
    }
    if out.is_empty() && !name.is_empty() {
        out.push((name.to_string(), false));
    }
    out
}

/// 对 name 做匹配并返回是否命中及高亮区间（仅对「名称」匹配，NOT 子句不高亮）
pub fn parse_and_match(
    query: &str,
    name: &str,
    is_dir: bool,
    config_override: Option<&MatchConfig>,
) -> Result<(bool, Vec<HighlightRange>), String> {
    let parsed = parse(query)?;
    let config = config_override.unwrap_or(&parsed.config);
    if !type_matches(config.type_filter, is_dir) {
        return Ok((false, vec![]));
    }
    let expr = match parsed.expr {
        Some(e) => e,
        None => return Ok((true, vec![])), // 空表达式视为全匹配
    };
    let (folded_name, map) = fold_for_match(name, config);
    let (matched, ranges) = eval_expr(&expr, name, &folded_name, &map, config)?;
    Ok((matched, ranges))
}

fn type_matches(filter: TypeFilter, is_dir: bool) -> bool {
    match filter {
        TypeFilter::Both => true,
        TypeFilter::FileOnly => !is_dir,
        TypeFilter::FolderOnly => is_dir,
    }
}

fn eval_expr(
    expr: &Expr,
    orig_name: &str,
    folded_name: &str,
    map: &[(usize, usize)],
    config: &MatchConfig,
) -> Result<(bool, Vec<HighlightRange>), String> {
    match expr {
        Expr::Or(children) => {
            for c in children {
                let (ok, r) = eval_expr(c, orig_name, folded_name, map, config)?;
                if ok {
                    return Ok((true, r));
                }
            }
            Ok((false, vec![]))
        }
        Expr::And(children) => {
            let mut all_ranges = Vec::new();
            for c in children {
                let (ok, r) = eval_expr(c, orig_name, folded_name, map, config)?;
                if !ok {
                    return Ok((false, vec![]));
                }
                all_ranges.extend(r);
            }
            Ok((true, merge_ranges(all_ranges)))
        }
        Expr::Not(inner) => {
            let (ok, _) = eval_expr(inner, orig_name, folded_name, map, config)?;
            Ok((!ok, vec![])) // NOT 不高亮
        }
        Expr::Term(t) => {
            let (folded_pat, _) = fold_for_match(t, config);
            if folded_pat.is_empty() {
                return Ok((true, vec![]));
            }
            if has_glob(t) {
                let _re = glob_to_regex(t).map_err(|e| e.to_string())?;
                let re_folded = glob_to_regex(&folded_pat).map_err(|e| e.to_string())?;
                if re_folded.is_match(folded_name) {
                    let ranges = find_regex_ranges(orig_name, folded_name, &re_folded, map);
                    return Ok((true, ranges));
                }
                Ok((false, vec![]))
            } else {
                let found = folded_name.contains(&folded_pat);
                let ranges = if found {
                    find_substring_ranges(orig_name, folded_name, &folded_pat, map)
                } else {
                    vec![]
                };
                Ok((found, ranges))
            }
        }
        Expr::Phrase(p) => {
            let (folded_pat, _) = fold_for_match(p, config);
            if folded_pat.is_empty() {
                return Ok((true, vec![]));
            }
            let found = folded_name.contains(&folded_pat);
            let ranges = if found {
                find_substring_ranges(orig_name, folded_name, &folded_pat, map)
            } else {
                vec![]
            };
            Ok((found, ranges))
        }
    }
}

fn merge_ranges(mut ranges: Vec<HighlightRange>) -> Vec<HighlightRange> {
    if ranges.is_empty() {
        return ranges;
    }
    ranges.sort_by_key(|r| r.start);
    let mut out = vec![ranges[0].clone()];
    for r in ranges.into_iter().skip(1) {
        let last = out.last_mut().unwrap();
        if r.start <= last.end {
            last.end = last.end.max(r.end);
        } else {
            out.push(r);
        }
    }
    out
}

/// 仅解析查询，得到配置与表达式（不执行匹配）
pub fn parse(query: &str) -> Result<ParsedQuery, String> {
    let tokens = tokenize(query)?;
    let (config, expr_tokens) = extract_modifiers(tokens);
    let expr = parse_expr(&expr_tokens)?;
    Ok(ParsedQuery {
        config,
        expr,
    })
}

#[derive(Debug, Clone)]
enum Token {
    Modifier(ModifierKind),
    Not,
    Or,
    Term(String),
    Phrase(String),
}

#[derive(Debug, Clone, Copy)]
enum ModifierKind {
    Case,
    Ascii,
    Diacritics,
    File,
    Folder,
}

fn tokenize(s: &str) -> Result<Vec<Token>, String> {
    let s = s.trim();
    let mut out = Vec::new();
    let mut i = 0;
    let chars: Vec<char> = s.chars().collect();
    while i < chars.len() {
        let c = chars[i];
        if c == '"' {
            let start = i + 1;
            i += 1;
            while i < chars.len() && chars[i] != '"' {
                i += 1;
            }
            if i >= chars.len() {
                return Err("未闭合的引号".to_string());
            }
            let phrase: String = chars[start..i].iter().collect();
            out.push(Token::Phrase(phrase));
            i += 1;
            continue;
        }
        if c == '!' {
            out.push(Token::Not);
            i += 1;
            continue;
        }
        if c == '|' {
            out.push(Token::Or);
            i += 1;
            continue;
        }
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && !chars[i].is_whitespace() && chars[i] != '"' && chars[i] != '!' && chars[i] != '|' {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();
        if word.ends_with(':') {
            let name = word.trim_end_matches(':').to_lowercase();
            let kind = match name.as_str() {
                "case" => ModifierKind::Case,
                "ascii" => ModifierKind::Ascii,
                "diacritics" => ModifierKind::Diacritics,
                "file" => ModifierKind::File,
                "folder" => ModifierKind::Folder,
                _ => {
                    out.push(Token::Term(word));
                    continue;
                }
            };
            out.push(Token::Modifier(kind));
        } else {
            out.push(Token::Term(word));
        }
    }
    Ok(out)
}

fn extract_modifiers(tokens: Vec<Token>) -> (MatchConfig, Vec<Token>) {
    let mut config = MatchConfig::default();
    let mut rest = Vec::new();
    for t in tokens {
        match t {
            Token::Modifier(k) => match k {
                ModifierKind::Case => config.case_sensitive = true,
                ModifierKind::Ascii => config.ascii_fold_only = true,
                ModifierKind::Diacritics => config.diacritics_sensitive = true,
                ModifierKind::File => config.type_filter = TypeFilter::FileOnly,
                ModifierKind::Folder => config.type_filter = TypeFilter::FolderOnly,
            },
            other => rest.push(other),
        }
    }
    (config, rest)
}

/// 解析表达式：优先级 ! > AND(隐式) > OR
fn parse_expr(tokens: &[Token]) -> Result<Option<Expr>, String> {
    if tokens.is_empty() {
        return Ok(None);
    }
    let or_parts: Vec<Vec<Token>> = split_by(tokens, |t| matches!(t, Token::Or))
        .into_iter()
        .filter(|p| !p.is_empty())
        .collect();
    if or_parts.is_empty() {
        return Ok(None);
    }
    if or_parts.len() > 1 {
        let mut exprs = Vec::new();
        for part in or_parts {
            exprs.push(parse_and_expr(&part)?);
        }
        return Ok(Some(Expr::Or(exprs)));
    }
    Ok(Some(parse_and_expr(&or_parts[0])?))
}

fn split_by<F>(tokens: &[Token], pred: F) -> Vec<Vec<Token>>
where
    F: Fn(&Token) -> bool,
{
    let mut out: Vec<Vec<Token>> = Vec::new();
    let mut cur = Vec::new();
    for t in tokens {
        if pred(t) {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(t.clone());
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// AND 为多个 not_expr 隐式连接；not_expr = [Not]* (Term | Phrase)
fn parse_and_expr(tokens: &[Token]) -> Result<Expr, String> {
    let mut exprs = Vec::new();
    let mut i = 0;
    while i < tokens.len() {
        let mut not_count = 0;
        while i < tokens.len() && matches!(tokens[i], Token::Not) {
            not_count += 1;
            i += 1;
        }
        if i >= tokens.len() {
            return Err("! 后缺少词或短语".to_string());
        }
        let atom = match &tokens[i] {
            Token::Term(s) => Expr::Term(s.clone()),
            Token::Phrase(s) => Expr::Phrase(s.clone()),
            _ => return Err("期望词或短语".to_string()),
        };
        i += 1;
        let mut e = atom;
        for _ in 0..not_count {
            e = Expr::Not(Box::new(e));
        }
        exprs.push(e);
    }
    if exprs.is_empty() {
        return Err("空表达式".to_string());
    }
    if exprs.len() == 1 {
        Ok(exprs.into_iter().next().unwrap())
    } else {
        Ok(Expr::And(exprs))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn match_name(query: &str, name: &str, is_dir: bool) -> Result<bool, String> {
        parse_and_match(query, name, is_dir, None).map(|(ok, _)| ok)
    }

    #[test]
    fn test_and() {
        assert!(match_name("foo bar", "foo_bar.txt", false).unwrap());
        assert!(match_name("foo bar", "bar_foo.x", false).unwrap());
        assert!(!match_name("foo bar", "foo_only", false).unwrap());
        assert!(!match_name("foo bar", "bar_only", false).unwrap());
    }

    #[test]
    fn test_or() {
        assert!(match_name("foo|bar", "foo.x", false).unwrap());
        assert!(match_name("foo|bar", "bar.x", false).unwrap());
        assert!(match_name("a|b", "a", false).unwrap());
        assert!(!match_name("foo|bar", "baz", false).unwrap());
    }

    #[test]
    fn test_not() {
        assert!(match_name("foo !bar", "foo.txt", false).unwrap());
        assert!(!match_name("foo !bar", "foobar", false).unwrap());
        assert!(!match_name("!bar", "bar", false).unwrap());
        assert!(match_name("!bar", "other", false).unwrap());
    }

    #[test]
    fn test_phrase() {
        assert!(match_name("\"foo bar\"", "foo bar.txt", false).unwrap());
        assert!(!match_name("\"foo bar\"", "foobar", false).unwrap());
    }

    #[test]
    fn test_glob() {
        assert!(match_name("a*b", "ab", false).unwrap());
        assert!(match_name("a*b", "axxb", false).unwrap());
        assert!(!match_name("a*b", "abx", false).unwrap());
        assert!(match_name("*.ts", "main.ts", false).unwrap());
        assert!(match_name("file?.txt", "file1.txt", false).unwrap());
    }

    #[test]
    fn test_case_insensitive_default() {
        assert!(match_name("Foo", "foo.txt", false).unwrap());
        assert!(match_name("FOO", "foo", false).unwrap());
    }

    #[test]
    fn test_type_filter() {
        assert!(match_name("x", "x", false).unwrap());
        assert!(match_name("x", "x", true).unwrap());
        assert!(match_name("file: x", "x", false).unwrap());
        assert!(!match_name("file: x", "x", true).unwrap());
        assert!(match_name("folder: x", "x", true).unwrap());
        assert!(!match_name("folder: x", "x", false).unwrap());
    }

    #[test]
    fn test_ranges_to_segments() {
        let segs = ranges_to_segments("hello world", &[0..5, 6..11]);
        assert_eq!(segs.len(), 3);
        assert_eq!(segs[0], ("hello".to_string(), true));
        assert_eq!(segs[1], (" ".to_string(), false));
        assert_eq!(segs[2], ("world".to_string(), true));
    }

    /// 中文等多字节 UTF-8：find() 返回字节偏移，需正确转为字符索引，避免越界 panic
    #[test]
    fn test_chinese_substring_match() {
        assert!(match_name("数据", "数据门户.txt", false).unwrap());
        assert!(match_name("数据门户", "数据门户.x", false).unwrap());
        assert!(match_name("门户", "数据门户", false).unwrap());
        assert!(!match_name("户数", "数据门户", false).unwrap());
    }
}
