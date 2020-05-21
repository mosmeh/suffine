use suffine::IndexBuilder;

fn main() {
    let text = "I scream, you scream, we all scream for ice cream!";
    let index_builder = IndexBuilder::new(text);
    let index = index_builder.build().unwrap();
    for i in itertools::sorted(index.find_positions("cream")) {
        println!("{}", i);
    }
}
