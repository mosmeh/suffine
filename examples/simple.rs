use suffine::IndexBuilder;

fn main() {
    let text = "I scream, you scream, we all scream for ice cream!";
    let index_builder = IndexBuilder::new(text);
    let _index = index_builder.build_in_memory();
}
