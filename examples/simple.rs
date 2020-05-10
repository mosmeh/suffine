use suffine::IndexBuilder;

fn main() {
    let mut index_builder = IndexBuilder::new();
    index_builder.add("I scream, you scream, we all scream for ice cream!");
    index_builder.add("She sells seashells by the seashore.");

    let index = index_builder.build();
    for (doc_id, pos) in index.search("cream") {
        println!("{}: {}", doc_id, pos)
    }
}
