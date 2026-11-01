use std::{collections::HashMap, format, fs::{self}, ops::Add, path::Path, println, string};


#[derive(Debug)]
enum Value {
    Char(char),
    Number(u32),
}


#[derive(Debug)]
struct Node {
    value: Value,
    left: Option<Box<Node>>,
    right: Option<Box<Node>>,
}

impl Node {
    fn new(value: Value) -> Self {
        Self {
            value,
            left: None,
            right: None,
        }
    }
    fn frequency(&self) -> u32 {
        match self.value {
            Value::Number(n) => n,
            Value::Char(_) => 0,
        }
    }
}

fn open_file(path: &Path) -> Result<String, String> {
    if !path.exists(){
        return Err("Impossible to map path".to_string());
    }

    let content = fs::read_to_string(path)
    .map_err(|e| format!("Erro while reading file: {}", e))?;

    return Ok(content);
}

fn fill_map(map: &mut HashMap<char, u32>, row: String){
    for character in row.chars() {
        *map.entry(character).or_insert(0) += 1;
    }
}

fn build_encoded_map(root: &Node)-> HashMap<char, String>{
    let mut encoding_map: HashMap<char, String> = HashMap::new();

    fn recursive(
        node: &Node, 
        code: &mut String,
        encoding_map: &mut HashMap<char, String>
    ){
        match node.value {
            Value::Char(char) => {
                encoding_map.insert(char, code.clone());
            },

            Value::Number(_) => {
                if let Some(left) = &node.left {
                    code.push('0');
                    recursive(left, code, encoding_map);
                    code.pop();
                }

                if let Some(right) = &node.right {
                    code.push('1');
                    recursive(right, code, encoding_map);
                    code.pop();
                }
            }
        }
    }

    let mut code = String::new();
    recursive(root, &mut code, &mut encoding_map);

    encoding_map
}

fn encode_content(content: String, map: &HashMap<char, String> ) -> Result<String, String>{
    let mut encodedContent = String::new();
    for character in content.chars(){
        let encoding = map.get(&character);

        if encoding.is_none(){
            return Err("Charactere nao reconhecido".to_string());
        }

        encodedContent += encoding.unwrap()
    }

    Ok(encodedContent)
}

fn decode_content(content: String, map: &HashMap<String, char> ) -> Result<String, String>{
    let mut decoded = String::new();
    let mut slice = String::new();

    for bit in content.chars(){
        slice.push(bit);

        if let Some(character) = map.get(&slice) {
            decoded.push(*character);
            slice.clear();
        }
    }

    if !slice.is_empty() {
        return Err("Invalid encoded content".to_string());
    }

    Ok(decoded)
}

fn main() {
    let mut letters_map: HashMap<char, u32> = HashMap::new();

    let input_path = Path::new("./input/hello.txt");

    let contents = open_file(input_path).unwrap();

    fill_map(&mut letters_map, contents.clone());

    let mut numberTuppleArr: Vec<(char, u32)> = Vec::new();

    while letters_map.len() > 0 {
        let smallest: Option<(char, u32)> = letters_map
            .iter()
            .min_by_key(|(_, value)| *value)
            .map(|(&key, &value)| (key, value));

        numberTuppleArr.push(smallest.unwrap());

        if let Some((character, count)) = smallest {
            letters_map.remove(&character);
        }
    }


    let mut node_arr: Vec<Node> = Vec::new();

    for index in 0..numberTuppleArr.len() - 1{
        let first = numberTuppleArr[index];
        let second = numberTuppleArr[index + 1];

        let mut root = Node::new(Value::Number(first.1 + second.1));

        root.left = Some(Box::new(Node::new(Value::Char(first.0))));
        root.right = Some(Box::new(Node::new(Value::Char(second.0))));

        node_arr.push(root);
    }

    while node_arr.len() > 1 {
        node_arr.sort_by_key(|node| node.frequency());

        let left = node_arr.remove(0);
        let right = node_arr.remove(0);

        let frequency = left.frequency() + right.frequency();

        let root = Node{
            value: Value::Number(frequency),
            left: Some(Box::new(left)),
            right: Some(Box::new(right))
        };

        node_arr.push(root);
    }

    let encode_map = build_encoded_map(&node_arr[0]);

    let result = encode_content(contents.clone(), &encode_map).unwrap();

    println!("{:?}", result);

    let decode_map: HashMap<String, char> = encode_map
    .iter()
    .map(|(&character, code)| (code.clone(), character))
    .collect();

    let decodedContent = decode_content(result.clone(), &decode_map).unwrap();

    println!("{}", decodedContent);


}
