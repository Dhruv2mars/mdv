#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictHunk {
    pub local_start: usize,
    pub external_start: usize,
    pub local_lines: Vec<String>,
    pub external_lines: Vec<String>,
}

#[derive(Clone, Copy)]
enum Op {
    Equal,
    Delete,
    Insert,
}

pub fn compute_conflict_hunks(local: &str, external: &str) -> Vec<ConflictHunk> {
    let local_lines = split_lines(local);
    let external_lines = split_lines(external);
    let ops = diff_ops(&local_lines, &external_lines);

    let mut hunks = Vec::new();
    let mut local_idx = 0usize;
    let mut external_idx = 0usize;
    let mut i = 0usize;

    while i < ops.len() {
        match ops[i] {
            Op::Equal => {
                local_idx += 1;
                external_idx += 1;
                i += 1;
            }
            Op::Delete | Op::Insert => {
                let start_local = local_idx;
                let start_external = external_idx;
                let mut local_chunk = Vec::new();
                let mut external_chunk = Vec::new();

                while i < ops.len() {
                    match ops[i] {
                        Op::Delete => {
                            local_chunk.push(local_lines[local_idx].clone());
                            local_idx += 1;
                            i += 1;
                        }
                        Op::Insert => {
                            external_chunk.push(external_lines[external_idx].clone());
                            external_idx += 1;
                            i += 1;
                        }
                        Op::Equal => break,
                    }
                }

                hunks.push(ConflictHunk {
                    local_start: start_local,
                    external_start: start_external,
                    local_lines: local_chunk,
                    external_lines: external_chunk,
                });
            }
        }
    }

    hunks
}

fn split_lines(text: &str) -> Vec<String> {
    text.split('\n').map(ToString::to_string).collect()
}

fn diff_ops(local: &[String], external: &[String]) -> Vec<Op> {
    let n = local.len();
    let m = external.len();
    let mut lcs = vec![vec![0usize; m + 1]; n + 1];

    for i in (0..n).rev() {
        for j in (0..m).rev() {
            lcs[i][j] = if local[i] == external[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut i = 0usize;
    let mut j = 0usize;
    let mut ops = Vec::new();

    while i < n && j < m {
        if local[i] == external[j] {
            ops.push(Op::Equal);
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            ops.push(Op::Delete);
            i += 1;
        } else {
            ops.push(Op::Insert);
            j += 1;
        }
    }

    while i < n {
        ops.push(Op::Delete);
        i += 1;
    }
    while j < m {
        ops.push(Op::Insert);
        j += 1;
    }

    ops
}

#[cfg(test)]
mod tests {
    use super::compute_conflict_hunks;

    #[test]
    fn returns_empty_for_identical_text() {
        assert!(compute_conflict_hunks("a\nb", "a\nb").is_empty());
    }

    #[test]
    fn returns_insert_delete_replace_hunks() {
        let hunks = compute_conflict_hunks("a\nb\nc", "a\nB\nc\nd");
        assert_eq!(hunks.len(), 2);
        assert_eq!(hunks[0].local_start, 1);
        assert_eq!(hunks[0].external_start, 1);
        assert_eq!(hunks[0].local_lines, vec!["b".to_string()]);
        assert_eq!(hunks[0].external_lines, vec!["B".to_string()]);
        assert!(hunks[1].local_lines.is_empty());
        assert_eq!(hunks[1].external_lines, vec!["d".to_string()]);
    }

    #[test]
    fn returns_trailing_delete_hunk() {
        let hunks = compute_conflict_hunks("a\nb\nc", "a");
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].local_start, 1);
        assert_eq!(hunks[0].external_start, 1);
        assert_eq!(hunks[0].local_lines, vec!["b".to_string(), "c".to_string()]);
        assert!(hunks[0].external_lines.is_empty());
    }
}
