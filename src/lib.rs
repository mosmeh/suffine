use suffix::SuffixTable;

pub struct Index {
    concatenated_str: String,
    doc_offsets: Vec<usize>,
    st: SuffixTable<'static, 'static>,
}

impl Index {
    pub fn search(&self, query: &str) -> Vec<(usize, usize)> {
        self.st
            .positions(&query)
            .iter()
            .map(|p| {
                let p = *p as usize;
                let doc_id = match self.doc_offsets.binary_search(&p) {
                    Ok(x) => x,
                    Err(x) => x - 1,
                };
                let pos_in_doc = p - self.doc_offsets[doc_id];
                (doc_id, pos_in_doc)
            })
            .collect::<Vec<(_, _)>>()
    }

    pub fn document(&self, doc_id: usize) -> Option<&str> {
        if doc_id < self.doc_offsets.len() {
            let begin = self.doc_offsets[doc_id];
            let end = if doc_id == self.doc_offsets.len() - 1 {
                self.concatenated_str.len() - 1
            } else {
                self.doc_offsets[doc_id + 1] - 1
            };
            Some(&self.concatenated_str[begin..end])
        } else {
            None
        }
    }
}

pub struct IndexBuilder {
    concatenated_str: String,
    doc_offsets: Vec<usize>,
}

impl IndexBuilder {
    pub fn new() -> Self {
        Default::default()
    }

    pub fn add(&mut self, text: &str) -> &mut Self {
        self.doc_offsets.push(self.concatenated_str.len());

        self.concatenated_str.push_str(&text);
        self.concatenated_str.push('\0');

        self
    }

    pub fn build(self) -> Index {
        let st = SuffixTable::new(self.concatenated_str.clone());
        Index {
            concatenated_str: self.concatenated_str,
            doc_offsets: self.doc_offsets,
            st,
        }
    }
}

impl Default for IndexBuilder {
    fn default() -> Self {
        Self {
            concatenated_str: "".to_string(),
            doc_offsets: Vec::new(),
        }
    }
}

#[cfg(test)]
#[macro_use(quickcheck)]
extern crate quickcheck_macros;

#[cfg(test)]
mod tests {
    use crate::IndexBuilder;

    #[quickcheck]
    fn single_document(text: String) {
        let mut index_builder = IndexBuilder::new();
        index_builder.add(&text);

        let index = index_builder.build();
        assert_eq!(index.search(""), vec![]);
        if !text.is_empty() {
            assert_eq!(index.search(&text), vec![(0, 0)]);
        }
        assert_eq!(index.document(0).unwrap(), text);
    }

    #[quickcheck]
    fn multiple_documents(texts: Vec<String>) {
        let mut index_builder = IndexBuilder::new();
        for text in &texts {
            if text.contains('\0') {
                return; // TODO
            }
            index_builder.add(&text);
        }

        let index = index_builder.build();
        assert_eq!(index.search(""), vec![]);
        for text in &texts {
            for (doc_id, pos) in index.search(&text) {
                let doc_text = index.document(doc_id).unwrap();
                assert_eq!(&doc_text[pos..pos + text.len()], text);
            }
        }
    }

    #[test]
    fn nonexistence() {
        let mut index_builder = IndexBuilder::new();
        index_builder.add("a");
        index_builder.add("b");

        let index = index_builder.build();
        assert_eq!(index.search("c"), vec![]);
        assert_eq!(index.document(5), None);
    }

    #[test]
    fn duplication() {
        let mut index_builder = IndexBuilder::new();
        index_builder.add("a");
        index_builder.add("a");
        index_builder.add("ab");

        let index = index_builder.build();
        let mut result = index.search("a");
        result.sort_unstable();
        assert_eq!(result, vec![(0, 0), (1, 0), (2, 0)]);
    }
}
