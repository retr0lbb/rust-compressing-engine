use std::{collections::HashMap, format, fs::{self, File, Metadata}, io::Write, ops::Add, path::{self, Path}, println, string, vec};
use clap::{ArgGroup, Parser};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "huffman", about = "Compressor de arquivos de texto com Huffman")]
#[command(group(ArgGroup::new("mode").required(true).args(["compress", "decompress"])))]
struct Cli {
    /// Arquivo de texto a comprimir
    #[arg(long, value_name = "ARQUIVO")]
    compress: Option<PathBuf>,

    /// Arquivo .huff a descomprimir
    #[arg(long, value_name = "ARQUIVO.huff")]
    decompress: Option<PathBuf>,

    /// Pasta de saída
    #[arg(short, long, value_name = "PASTA", default_value = "./output")]
    output: PathBuf,
}

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

struct DocumentHeader {
    magic_bytes: [u8; 4],
    original_file_size: u64,
    binary_tree: Vec<u8>,
    binary_tree_size: u32,
    encoded_bit_size: u64,
}
struct DocumentBody{
    binary_content: Vec<u8>
}

fn open_file(path: &Path) -> Result<(String, Metadata), String> {
    if !path.exists(){
        return Err("Impossible to map path".to_string());
    }

    let content = fs::read_to_string(path)
    .map_err(|e| format!("Erro while reading file: {}", e))?;

    let metadata = fs::metadata(path).unwrap();

    return Ok((content, metadata));
}

fn build_output_path(input: &Path, output_dir: &Path, extension: &str) -> Result<PathBuf, String> {
    let stem = input
        .file_stem()
        .ok_or("Invalid input file name")?
        .to_string_lossy();

    fs::create_dir_all(output_dir)
        .map_err(|e| format!("Could not create output dir: {}", e))?;

    Ok(output_dir.join(format!("{}.{}", stem, extension)))
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

fn encode_content(content: String, map: &HashMap<char, String> ) -> Result<(Vec<u8>, u64), String>{
    let mut encode_content_byts: Vec<u8> = Vec::new();
    let mut current_bytes: u8 = 0;
    let mut bit_count:u8 = 0;
    let mut total_bits: u64 = 0;

    for character in content.chars(){
        let encoding = map.get(&character)
            .ok_or("Character nao reconhecido")?;

        for bit in encoding.chars(){
            current_bytes <<= 1;

            match bit {
                '0' =>{}
                '1' =>{
                    current_bytes |= 1
                },
                _ => return Err("Bit invalido no mapa".to_string()),
            }

            bit_count += 1;
            total_bits += 1;


            if bit_count == 8 {
                encode_content_byts.push(current_bytes);
                current_bytes = 0;
                bit_count = 0;
            }
        }
    }
    if bit_count > 0 {
        current_bytes <<= 8 - bit_count;
        encode_content_byts.push(current_bytes);
    }

    Ok((encode_content_byts, total_bits))
}

fn transform_binary_tree_in_bytes(node: &Node, output: &mut Vec<u8>) -> Result<(), String>{
    match &node.value {
        Value::Char(character) => {
            output.push(1);

            let character_bytes = (*character as u32).to_be_bytes();
            output.extend_from_slice(&character_bytes);
        }
        Value::Number(_) => {
            output.push(0);

            let left = node.left.as_deref().ok_or("Missing left child");
            let right = node.right.as_deref().ok_or("Right child missing");

            transform_binary_tree_in_bytes(left?, output)?;
            transform_binary_tree_in_bytes(right?, output)?;
        }
    }

    Ok(())
}

fn serialize_huffman_document(header: &DocumentHeader, body: &DocumentBody) -> Vec<u8>{
    let mut document: Vec<u8> = Vec::new();

    //1.magic 2.original_file_size 3.tree size 4.encoded_bit_size
    document.extend_from_slice(&header.magic_bytes);
    document.extend_from_slice(&header.original_file_size.to_be_bytes());
    document.extend_from_slice(&header.binary_tree_size.to_be_bytes());
    document.extend_from_slice(&header.encoded_bit_size.to_be_bytes());


    //1. tree, 2.body
    document.extend_from_slice(&header.binary_tree);
    document.extend_from_slice(&body.binary_content);


    return document
}

fn save_huffman_document(doc_bin: Vec<u8>, exit_path: &Path) {

    let mut huffman_file = File::create(exit_path).unwrap();

    huffman_file.write_all(&doc_bin);
}

fn save_normal_file(content: String, exit_path: &Path){
    let mut huffman_file = File::create(exit_path).unwrap();

    huffman_file.write(content.as_bytes());
}

fn open_compressed_file(path: &Path)-> Result<Vec<u8>, String>{
    fs::read(path).map_err(|error| error.to_string())
}

fn desserialize_document(document: &[u8]) -> Result<(Vec<u8>, Vec<u8>, u64, u64), String>{
    let magic_bytes = &document[0..4];

    if magic_bytes != [72, 85, 70, 70] {
        return Err("Not encoded with HUFFMAN ALG".to_string());
    }

    let original_file_size = u64::from_be_bytes(
        document[4..12].try_into().unwrap()
    );

    let tree_size = u32::from_be_bytes(
        document[12..16].try_into().unwrap()
    );

    let encoding_bit_size = u64::from_be_bytes(
        document[16..24].try_into().unwrap()
    );

    let tree_start = 24;
    let tree_end = tree_start + tree_size as usize;

    let binary_tree = &document[tree_start..tree_end];



    let encoded_byte_size = (encoding_bit_size + 7) / 8;

    let content_start = tree_end.clone();
    let content_end = content_start + encoded_byte_size as usize;
    let binary_content = &document[content_start..content_end];

    Ok((binary_tree.to_vec(), binary_content.to_vec(), encoding_bit_size, original_file_size))
}

fn desserialize_content(
    serialized_content: &[u8],
    encoded_bit_size: u64,
    root: &Node,
) -> Result<String, String> {
    let mut result = String::new();
    let mut current_node = root;

    for bit_index in 0..encoded_bit_size {
        // Descobre qual byte contém o bit
        let byte_index = (bit_index / 8) as usize;

        // Descobre a posição do bit dentro do byte (MSB primeiro)
        let bit_position = 7 - (bit_index % 8);

        let byte = *serialized_content
            .get(byte_index)
            .ok_or("Unexpected end of content")?;

        let bit = (byte >> bit_position) & 1;

        // Caminha pela árvore
        current_node = match bit {
            0 => current_node.left.as_deref()
                .ok_or("Missing left child")?,
            1 => current_node.right.as_deref()
                .ok_or("Missing right child")?,
            _ => unreachable!(),
        };

        // Verifica se chegou em uma folha
        match &current_node.value {
            Value::Char(character) => {
                result.push(*character);

                // Reinicia o percurso na raiz
                current_node = root;
            }
            Value::Number(_) => {}
        }
    }

    // Verifica se terminou no meio de um código Huffman
    if !std::ptr::eq(current_node, root) {
        return Err("Incomplete Huffman code".to_string());
    }

    Ok(result)
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

fn deserialize_tree(
    s_tree: &[u8],
    position: &mut usize,
) -> Result<Node, String> {

    if *position >= s_tree.len() {
        return Err("Unexpected end of tree".to_string());
    }

    let number = s_tree[*position];

    *position += 1;

    match number {
        // Nó interno
        0 => {
            let left = deserialize_tree(s_tree, position)?;
            let right = deserialize_tree(s_tree, position)?;

            Ok(Node {
                value: Value::Number(0),
                left: Some(Box::new(left)),
                right: Some(Box::new(right)),
            })
        }

        // Folha
        1 => {
            let end = (*position)
                .checked_add(4)
                .ok_or("Invalid position")?;

            let char_bytes: [u8; 4] = s_tree
                .get(*position..end)
                .ok_or("Invalid character bytes")?
                .try_into()
                .map_err(|_| "Invalid character bytes")?;

            *position = end;

            let char_code = u32::from_be_bytes(char_bytes);

            let character = char::from_u32(char_code)
                .ok_or("Invalid character")?;

            Ok(Node {
                value: Value::Char(character),
                left: None,
                right: None,
            })
        }

        _ => Err("Invalid node marker".to_string()),
    }
}

fn build_huffman_tree(content: &str) -> Result<Node, String> {
    let mut letters_map: HashMap<char, u32> = HashMap::new();
    fill_map(&mut letters_map, content.to_owned());

    let mut numberTuppleArr: Vec<(char, u32)> = Vec::new();

    while letters_map.len() > 0 {
        let smallest: Option<(char, u32)> = letters_map
            .iter()
            .min_by_key(|(_, value)| *value)
            .map(|(&key, &value)| (key, value));

        numberTuppleArr.push(smallest.unwrap());

        if let Some((character, _)) = smallest {
            letters_map.remove(&character);
        }
    }

    let mut node_arr: Vec<Node> = Vec::new();

    for index in 0..numberTuppleArr.len().saturating_sub(1) {
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

        node_arr.push(Node {
            value: Value::Number(frequency),
            left: Some(Box::new(left)),
            right: Some(Box::new(right)),
        });
    }

    node_arr.into_iter().next().ok_or("Empty content".to_string())
}

fn decompress_document(document: &[u8]) -> Result<String, String> {
    let (serialized_tree, serialized_content, bit_size, original_size) =
        desserialize_document(document)?;

    let mut position = 0;
    let root = deserialize_tree(&serialized_tree, &mut position)?;

    if position != serialized_tree.len() {
        return Err("Corrupted tree".to_string());
    }

    let content = desserialize_content(&serialized_content, bit_size, &root)?;

    if content.len() as u64 != original_size {
        return Err("Decompressed size differs from header".to_string());
    }

    Ok(content)
}

fn verify_integrity(document: &[u8], original: &[u8]) -> Result<(), String> {
    let restored = decompress_document(document)?;

    if restored.as_bytes() != original {
        return Err("Integrity check failed: decompressed content differs from original".to_string());
    }

    Ok(())
}

fn compress_file(input: &Path, output_dir: &Path) -> Result<(PathBuf, u64, u64), String> {
    let (content, metadata) = open_file(input)?;
    let root = build_huffman_tree(&content)?;

    let mut tree_bytes: Vec<u8> = Vec::new();
    transform_binary_tree_in_bytes(&root, &mut tree_bytes)?;
    let tree_size = u32::try_from(tree_bytes.len())
        .map_err(|_| "Tree too large".to_string())?;

    let encode_map = build_encoded_map(&root);
    let (encoded, encoded_bit_size) = encode_content(content.clone(), &encode_map)?;

    let document = serialize_huffman_document(
        &DocumentHeader {
            magic_bytes: [72, 85, 70, 70], // HUFF
            original_file_size: metadata.len(),
            binary_tree: tree_bytes,
            binary_tree_size: tree_size,
            encoded_bit_size,
        },
        &DocumentBody { binary_content: encoded },
    );

    verify_integrity(&document, content.as_bytes())?;

    let compressed_size = document.len() as u64;
    let exit_path = build_output_path(input, output_dir, "huff")?;
    save_huffman_document(document, &exit_path);

    Ok((exit_path, metadata.len(), compressed_size))
}

fn decompress_file(input: &Path, output_dir: &Path) -> Result<(PathBuf, u64, u64), String> {
    let document = open_compressed_file(input)?;
    let content = decompress_document(&document)?;

    let original_size = content.len() as u64;
    let exit_path = build_output_path(input, output_dir, "txt")?;
    save_normal_file(content, &exit_path);

    Ok((exit_path, original_size, document.len() as u64))
}

fn print_size_table(original: u64, compressed: u64) {
    let reduction = if original == 0 {
        0.0
    } else {
        (1.0 - compressed as f64 / original as f64) * 100.0
    };

    println!("┌─────────────────┬────────────────┐");
    println!("│ {:<15} │ {:>14} │", "Original", format!("{} B", original));
    println!("│ {:<15} │ {:>14} │", "Comprimido", format!("{} B", compressed));
    println!("│ {:<15} │ {:>14} │", "Taxa de redução", format!("{:.2}%", reduction));
    println!("└─────────────────┴────────────────┘");
}

fn main() {
    let mut letters_map: HashMap<char, u32> = HashMap::new();

    let input_path = Path::new("./input/hello.txt");

    let contents = open_file(input_path).unwrap();

    fill_map(&mut letters_map, contents.0.clone());

    let root_tree_node = build_huffman_tree(&contents.0).unwrap();

    //B-Tree
    let mut tree_bytes: Vec<u8> = Vec::new();
    let _ = transform_binary_tree_in_bytes(&root_tree_node, &mut tree_bytes);
    let tree_size = u32::try_from(tree_bytes.len())
        .map_err(|_| "Tree too large");

    let encode_map = build_encoded_map(&root_tree_node);

    let (result, result_size) = encode_content(contents.0.clone(), &encode_map).unwrap();
    let encoded_bit_size = result_size;

    let document: Vec<u8> = serialize_huffman_document(
        &DocumentHeader { 
            magic_bytes: [72, 85, 70, 70], //HUFF 
            original_file_size: contents.1.len(), 
            binary_tree: tree_bytes, 
            binary_tree_size: tree_size.unwrap(), 
            encoded_bit_size },
            &DocumentBody { 
                binary_content: result
            }
        );

    let exit_path = Path::new("./output/compressed.huff");
    save_huffman_document(document, &exit_path);

    let compressed_file = open_compressed_file(exit_path).unwrap();

    let (serialized_tree, serialized_content, new_bit_size, original_file_size) = desserialize_document(&compressed_file).unwrap();

    let mut position = 0;

    let root = deserialize_tree(&serialized_tree, &mut position).unwrap();
    assert_eq!(position, serialized_tree.len());

    let mut new_encode_map = build_encoded_map(&root);

    let new_encode_map_reversed: HashMap<String, char> = new_encode_map
        .iter()
        .map(|(&character, code)| (code.clone(), character))
        .collect();


    // let result_2 = desserialize_content(&serialized_content, new_bit_size, &root); JEITO MAIS NERD DE FAZER ISSO

    let mut binary_string = String::new();

    for bit_index in 0..encoded_bit_size {
        let byte_index = (bit_index / 8) as usize;
        let bit_position = 7 - (bit_index % 8);

        let byte = serialized_content[byte_index];

        let bit = (byte >> bit_position) & 1;

        binary_string.push(if bit == 0 { '0' } else { '1' });
    }


    let result_3 = decode_content(binary_string, &new_encode_map_reversed).unwrap();

    let exit_path = Path::new("./output/decompressed.txt");
    save_normal_file(result_3, exit_path)

}

