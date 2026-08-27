use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct SourceId(pub usize);

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub name: PathBuf,
    pub source: String,
    line_starts: Vec<usize>,
}

impl SourceFile {
    fn new(name: PathBuf, source: String) -> Self {
        let mut line_starts = vec![0];
        line_starts.extend(source.match_indices('\n').map(|(index, _)| index + 1));
        Self { name, source, line_starts }
    }

    pub fn line_column(&self, offset: usize) -> (usize, usize) {
        let offset = offset.min(self.source.len());
        let line = self.line_starts.partition_point(|start| *start <= offset).saturating_sub(1);
        let column = self.source[self.line_starts[line]..offset].chars().count();
        (line + 1, column + 1)
    }

    pub fn line(&self, one_based: usize) -> Option<&str> {
        let start = *self.line_starts.get(one_based.checked_sub(1)?)?;
        let end = self.source[start..]
            .find('\n')
            .map(|relative| start + relative)
            .unwrap_or(self.source.len());
        Some(&self.source[start..end])
    }

    pub fn line_start(&self, one_based: usize) -> Option<usize> {
        self.line_starts.get(one_based.checked_sub(1)?).copied()
    }
}

#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    files: Vec<SourceFile>,
}

impl SourceMap {
    pub fn add(&mut self, name: impl AsRef<Path>, source: String) -> SourceId {
        let requested = name.as_ref();
        let requested_canonical = std::fs::canonicalize(requested).ok();
        if let Some((index, _)) = self.files.iter().enumerate().find(|(_, file)| {
            file.name == requested
                || requested_canonical.as_ref().is_some_and(|requested| {
                    std::fs::canonicalize(&file.name).ok().as_ref() == Some(requested)
                })
        }) {
            return SourceId(index);
        }
        let id = SourceId(self.files.len());
        self.files.push(SourceFile::new(requested.to_path_buf(), source));
        id
    }

    pub fn get(&self, id: SourceId) -> Option<&SourceFile> {
        self.files.get(id.0)
    }

    pub fn locate_span(&self, span: crate::token::Span, needle: Option<&str>) -> Option<SourceId> {
        let candidates: Vec<_> = self.files.iter().enumerate().filter(|(_, file)| {
            let Some(text) = file.source.get(span.start..span.end) else { return false };
            needle.is_none_or(|needle| text == needle || text.contains(needle))
        }).map(|(index, _)| SourceId(index)).collect();
        (candidates.len() == 1).then(|| candidates[0])
    }
}
