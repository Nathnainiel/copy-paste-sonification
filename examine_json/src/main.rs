extern crate serde;
extern crate serde_json;
use serde::{Deserialize, Serialize};
use std::io::prelude::*;
use std::fs::File;
use std::io::Read;
use std::io;
use std::collections::HashMap;

// deserialize into this struct
#[derive(Serialize, Deserialize)]
struct NodeRepresentation {
    source: String,
    name: String,
    children: Vec<NodeRepresentation>,
}

// transfer data and compute additional data into this struct
struct NodeData {
    source: String,
    name: String,
    children: Vec<usize>, // indexes to children
    deep_children: Vec<usize>, // indexes to all child nodes recursively
    siblings: Vec<usize>, // indexes to siblings
    depth: i32,
    index: usize,
    relevance: i32,
}

impl NodeData {
    pub fn new(node_rep: &NodeRepresentation) -> Self {
        NodeData {
            source: node_rep.source.clone(),
            name: node_rep.name.clone(),
            children: Vec::new(),
            deep_children: Vec::new(),
            siblings: Vec::new(),
            depth: 0,
            index: 0,
            relevance: 1,
        }
    }
    
    pub fn calculate_relevance(&mut self, word_map: &HashMap<String, i32>) {
        let words = split_into_words(&self.name);
        for word in words {
            self.relevance += match &*word {
                "text" | "Text" => 3,
                "binding" => 2,
                "KeyHandler" | "getKeyHandler" => 5,
                "KeyCodeCombination" | "KeyCombination" => 10,
                _ => 0,
            }
        }
    }
}

fn main() {
    let file = File::open("data.json")
        .expect("file should open read only");
    
    let mut data_tree: NodeRepresentation = serde_json::from_reader(file)
        .expect("file should be proper JSON");
    
    
    
    // rewrite data in a different structure
    write_data_to_file(&data_tree);
    
    let mut data_list: Vec<NodeData> = Vec::new(); // hold the new data representation
    let mut depth_map: HashMap<i32, Vec<usize>> = HashMap::new();
    // HashMap to count words
    let mut word_map: HashMap<String, i32> = HashMap::new();
    count_words_start(&data_tree, &mut word_map);
    
    convert_data(&data_tree, &mut data_list, &mut depth_map);
    
    // calculate relevance
    for node in &mut data_list {
        node.calculate_relevance(&word_map);
    }
    
    write_depth_data(&depth_map);
    
    write_score1(&data_list);
    
    // Display data
    loop {
        let mut input = String::new();
        print!("Show depth level: ");
        let _=io::stdout().flush();
        io::stdin().read_line(&mut input);
        let depth = input.trim().parse::<i32>().expect("Can't parse integer");
        let index_list = depth_map.get(&depth).unwrap();
        for index in index_list {
            println!("{}", &data_list[*index].name);
        }
    }
}

fn write_score1(data_list: &Vec<NodeData>) {
    let mut out_file = File::create("score1.csv").expect("Could not open file");
    
    for node in data_list {
        writeln!(out_file, "{},{},{} ", node.depth, node.source, node.relevance).expect("Could not write to file");
    }
    
}

fn write_depth_data(depth_map: &HashMap<i32, Vec<usize>>) {
    let mut out_file = File::create("depth_wave.txt").expect("Could not open file");
    let mut depth_index: i32 = 0;
    
    loop {
        let indexes = depth_map.get(&depth_index);
        match indexes {
            Some(indexes) => {
                let mut out_str = String::new();
                for _ in 0..indexes.len() {
                    out_str.push('>');
                }
                writeln!(out_file, "{}", out_str).expect("Could not write to file");
            },
            None => break
        }
        depth_index += 1;
    }
}

// Convert from NodeRepresentation to NodeData in a flat Vec
fn convert_data(data_tree: &NodeRepresentation, 
                data_list: &mut Vec<NodeData>, 
                depth_map: &mut HashMap<i32, Vec<usize>>) {
    
    traverse_convert(data_tree, data_list, depth_map, 0);
    // add all siblings from the depth_map
    add_siblings(data_list, depth_map);
}

fn traverse_convert(data_tree: &NodeRepresentation, 
                    data_list: &mut Vec<NodeData>, 
                    depth_map: &mut HashMap<i32, Vec<usize>>,
                    depth: i32,
                ) -> Vec<usize> // returns all indexes of children
{
    let mut node = NodeData::new(data_tree);
    node.depth = depth;
    data_list.push(node);
    let my_index = data_list.len()-1;
    data_list[my_index].index = my_index;
    let mut indexes = vec![my_index];
    // add my index to the appropriate depth_map Vec
    let depth_vec = depth_map.entry(depth).or_insert(Vec::new());
    depth_vec.push(my_index);
    // do recursively for all child nodes
    for child in &data_tree.children {
        let new_indexes = traverse_convert(&child, data_list, depth_map, depth + 1);
        match new_indexes.first() {
            Some(v) => data_list[my_index].children.push(*v),
            _ => ()
        }
        data_list[my_index].deep_children.extend(&new_indexes);
        indexes.extend(&new_indexes);
    }
    indexes
}

// Add siblings to NodeData instances
fn add_siblings(data_list: &mut Vec<NodeData>, depth_map: &mut HashMap<i32, Vec<usize>>) {
    for node in data_list {
        if let Some(sibling_list) = depth_map.get(&node.depth) {
            node.siblings = sibling_list.clone();
            // remove myself from the list of siblings
            let my_sibling_index = node.siblings.iter().position(|&x| x == node.index).unwrap();
            node.siblings.remove(my_sibling_index);
        }
    }
}

// Count the number of occurrences of every word and write to a file
fn count_words_start(data_tree: &NodeRepresentation, map: &mut HashMap<String, i32>) {
    
    count_words(data_tree, map);
    
    // convert HashMap to Vec
    let mut word_vec: Vec<(String, &i32)> = map
        .iter()
        .map(|(name, count)| (name.clone(), count))
        .collect();
    
    word_vec.sort_unstable_by(|a, b| b.1.cmp(a.1)); // sort by number of occurences
    
    // write results to a file
    let mut out_file = File::create("word_count.txt").expect("Could not open file");
    
    for tuple in word_vec {
        // println!("{}: {}", tuple.0, tuple.1);
        writeln!(out_file, "{}: {}", tuple.0, tuple.1).expect("Could not write to file");
    }
}

// count the amount of times a single word occurs in any name
fn count_words(data_tree: &NodeRepresentation, map: &mut HashMap<String, i32>) {
    let words = split_into_words(&data_tree.name);
        
    // add to HashMap, if it is not already added, and increase counter
    for word in words {
        if word != "" { // don't count empty strings
            let count = map.entry(word).or_insert(0);
            *count += 1;
        }
    }
    // do recursively for all child nodes
    for child in &data_tree.children {
        count_words(&child, map);
    }
}

fn split_into_words(string: &String) -> Vec<String> {
    let words: Vec<String> = string
        // add more split signs using the closure
        .split(|c| c == '.' || c == '(' || c == ')' || c == '$' || c == ',' || c == ' ') 
        .map(|c| c.to_owned()) // convert from &str to String
        .collect();
    words
}

// Write the data to a new file in a different representation
fn write_data_to_file(data_tree: &NodeRepresentation) {
    let mut out_file = File::create("data_restyled.txt").expect("Could not open file");
    
    let mut output_data: Vec<String> = Vec::new();
    traverse_data_tree(&mut output_data, data_tree, 0);
    
    for line in output_data {
        let mut new_line = line.clone();
        new_line.push('\n');
        out_file.write_all(new_line.as_bytes()).expect("Could not write to file");
    }
    
}

// recursively traverse data tree collecting strings
fn traverse_data_tree(output_data: &mut Vec<String>, data_tree: &NodeRepresentation, tree_depth: i32){
    let mut out_string = String::new();
    for _ in 0..tree_depth {
        out_string.push('>');
    }
    out_string.push(' ');
    // convert name to &str (&* dereferences to str) and add
    out_string.push_str(&*data_tree.name.clone());
    out_string.push_str(&*format!("-> {}", data_tree.children.len()));
    output_data.push(out_string);
    
    // do recursively for all child nodes
    for child in &data_tree.children {
        traverse_data_tree(output_data, &child, tree_depth + 1);
    }
}