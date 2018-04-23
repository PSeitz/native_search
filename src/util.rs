use fnv::FnvHashMap;
use parking_lot::Mutex;
use regex::Regex;
use serde_json;
use serde_json::{StreamDeserializer, Value};
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::SeekFrom;
use std::mem;
use std::mem::transmute;

use std;

pub fn normalize_text(text: &str) -> String {
    lazy_static! {
        static ref REGEXES:Vec<(Regex, & 'static str)> = vec![
            (Regex::new(r"\([fmn\d]\)").unwrap(), " "),
            (Regex::new(r"[\(\)]").unwrap(), " "),  // remove braces
            (Regex::new(r#"[{}'"“]"#).unwrap(), ""), // remove ' " {}
            (Regex::new(r"\s\s+").unwrap(), " "), // replace tabs, newlines, double spaces with single spaces
            (Regex::new(r"[,.…;・’-]").unwrap(), "")  // remove , .;・’-
        ];

    }
    let mut new_str = text.to_owned();
    // for tupl in &*RElet tupl = &&*REGEXES;
    for ref tupl in &*REGEXES {
        new_str = (tupl.0).replace_all(&new_str, tupl.1).into_owned();
    }

    new_str.to_lowercase().trim().to_owned()
}

use search::Hit;

pub fn get_bit_at(input: u32, n: u8) -> bool {
    if n < 32 {
        input & (1 << n) != 0
    } else {
        false
    }
}

#[inline]
pub fn set_bit_at(input: &mut u32, n: u8) {
    *input = *input | (1 << n)
}

const ONLY_HIGH_BIT_SET: u32 = (1 << 31);
const ALL_BITS_BUT_HIGHEST_SET: u32 = (1 << 31) - 1;

#[inline]
pub fn set_high_bit(input: &mut u32) {
    *input = *input | ONLY_HIGH_BIT_SET
}
#[inline]
pub fn unset_high_bit(input: &mut u32) {
    *input = *input & ALL_BITS_BUT_HIGHEST_SET
}

#[inline]
pub fn is_hight_bit_set(input: u32) -> bool {
    input & ONLY_HIGH_BIT_SET != 0
}

pub fn get_u32_from_bytes(data: &[u8], pos: usize) -> u32 {
    let mut bytes: [u8; 4] = [0, 0, 0, 0];
    bytes.copy_from_slice(&data[pos..pos + 4]);
    unsafe { transmute(bytes) }
}

#[inline]
pub fn unsafe_increase_len<T>(vec: &mut Vec<T>, add: usize) -> usize {
    vec.reserve(1 + add);
    let curr_pos = vec.len();
    unsafe {
        vec.set_len(curr_pos + add);
    }
    curr_pos
}

pub fn hits_map_to_vec(hits: &FnvHashMap<u32, f32>) -> Vec<Hit> {
    hits.iter().map(|(id, score)| Hit { id: *id, score: *score }).collect()
}

pub fn hits_vec_to_map(vec_hits: Vec<Hit>) -> FnvHashMap<u32, f32> {
    let mut hits: FnvHashMap<u32, f32> = FnvHashMap::default();
    for hit in vec_hits {
        hits.insert(hit.id, hit.score);
    }
    hits
}

pub fn boost_path(path: &str) -> (String, String) {
    concat_tuple(path, ".boost.subObjId", ".boost.value")
}

pub fn iter_json_stream<'a, F, T>(data: StreamDeserializer<'a, T, Value>, cb: &mut F)
where
    F: FnMut(&serde_json::Value),
    T: serde_json::de::Read<'a>,
{
    for el in data {
        if let Some(arr) = el.as_ref().unwrap().as_array() {
            for el in arr.iter() {
                cb(el);
            }
        } else {
            cb(el.as_ref().unwrap());
        }
    }
}

// #[test]
// fn concatooo() {
//     concato("nice", "cooel");
//     concato("nice".to_string(), "cooel");
//     let yop = "nice".to_string();
//     concato(&yop, "cooel");
// }

// trait IntoString {
//     fn into(&'a self) -> String;
// }

pub trait IntoString: Sized {
    fn into_string(self) -> String;
}

impl<'a> IntoString for &'a String {
    fn into_string(self) -> String {
        self.to_string()
    }
}
impl<'a, 'b> IntoString for &'a &'b String {
    fn into_string(self) -> String {
        self.to_string()
    }
}
impl<'a> IntoString for &'a str {
    fn into_string(self) -> String {
        self.to_string()
    }
}
impl<'a, 'b> IntoString for &'a &'b str {
    fn into_string(self) -> String {
        self.to_string()
    }
}
impl IntoString for String {
    fn into_string(self) -> String {
        self
    }
}

pub fn concat<S: IntoString + AsRef<str>>(path: S, suffix: &str) -> String {
    path.as_ref().into_string() + suffix
}

// pub fn concat(path: &str, suffix: &str) -> String {
//     path.to_string() + suffix
// }

pub fn get_file_path(folder: &str, path: &str) -> String {
    folder.to_string() + "/" + path
}

pub fn concat_tuple(path: &str, suffix: &str, suffix2: &str) -> (String, String) {
    (concat(path, suffix), concat(path, suffix2))
}

pub fn get_file_path_name(path_to_anchor: &str, is_text_index_part: bool) -> String {
    let suffix = if is_text_index_part { ".textindex" } else { "" };
    path_to_anchor.to_owned() + suffix
}

pub fn file_as_string(path: &str) -> Result<(String), io::Error> {
    info!("Loading File {}", path);
    let mut file = File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    Ok(contents)
}

pub fn get_level(path: &str) -> u32 {
    path.matches("[]").count() as u32
}

pub fn remove_array_marker(path: &str) -> String {
    path.split('.')
        .collect::<Vec<_>>()
        .iter()
        .map(|el| if el.ends_with("[]") { &el[0..el.len() - 2] } else { el })
        .collect::<Vec<_>>()
        .join(".")
}

#[inline]
pub fn vec_with_size_uninitialized<T>(size: usize) -> Vec<T> {
    let mut buffer = Vec::with_capacity(size);
    unsafe {
        buffer.set_len(size);
    }
    buffer
}

#[inline]
pub fn get_my_data_danger_zooone(start: u32, end: u32, data_file: &Mutex<fs::File>) -> Vec<u32> {
    let mut data: Vec<u32> = vec_with_size_uninitialized(end as usize - start as usize);
    {
        let p = data.as_mut_ptr();
        let len = data.len();
        let cap = data.capacity();

        unsafe {
            // complete control of the allocation to which `p` points.
            let ptr = std::mem::transmute::<*mut u32, *mut u8>(p);
            let mut data_bytes = Vec::from_raw_parts(ptr, len * 4, cap);

            load_bytes_into(&mut data_bytes, &*data_file.lock(), start as u64 * 4); //READ directly into u32 data

            // forget about temp data_bytes: no destructor run, so we can use data again
            mem::forget(data_bytes);
        }
    }
    data.retain(|el| *el != std::u32::MAX);
    data
}

#[inline]
pub fn load_bytes_into(buffer: &mut [u8], mut file: &File, offset: u64) {
    // @Temporary Use Result
    file.seek(SeekFrom::Start(offset)).unwrap();
    file.read_exact(buffer).unwrap();
}

#[inline]
pub fn extract_field_name(field: &str) -> String {
    field
    .chars()
    .take(field.chars().count() - 10) //remove .textindex
    .into_iter()
    .collect()
}

pub fn extract_prop_name(path: &str) -> &str {
    path.split('.')
        .map(|el| if el.ends_with("[]") { &el[0..el.len() - 2] } else { el })
        .filter(|el| *el != "textindex")
        .last()
        .expect(&format!("could not extract prop name from path {:?}", path))
}

#[inline]
pub fn get_steps_to_anchor(path: &str) -> Vec<String> {
    let mut paths = vec![];
    let mut current = vec![];
    let parts = path.split('.');

    for part in parts {
        current.push(part.to_string());
        if part.ends_with("[]") {
            let joined = current.join(".");
            paths.push(joined);
        }
    }

    paths.push(path.to_string() + ".textindex"); // add path to index
    paths
}

#[allow(unused_macros)]
macro_rules! print_json {
    ($e:expr) => {
        println!("{}", serde_json::to_string(&$e).unwrap());
    };
}

/// Also includes for e.g {"meaning":{"ger":["aye"]}}
/// the [meaning] and [meaning, ger] step, which is skipped in a search (not needed)
#[inline]
pub fn get_all_steps_to_anchor(path: &str) -> Vec<String> {
    let mut paths = vec![];
    let mut current = vec![];
    let parts = path.split('.');

    for part in parts {
        current.push(part.to_string());
        let joined = current.join(".");
        paths.push(joined);
    }

    // paths.push(path.to_string() + ".textindex"); // add path to index
    paths
}

use itertools::Itertools;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, PartialEq)]
pub enum NodeTree {
    Map(HashMap<String, NodeTree>),
    IsLeaf,
}

impl NodeTree {
    pub fn new(map: HashMap<String, NodeTree>) -> NodeTree {
        NodeTree::Map(map)
    }
}

pub fn to_node_tree(mut paths: Vec<Vec<String>>) -> NodeTree {
    paths.sort_by_key(|el| el[0].clone()); // sort for group_by
    let mut next = HashMap::default();
    for (key, group) in &paths.into_iter().group_by(|el| el.get(0).map(|el| el.clone())) {
        let key = key.unwrap();
        let mut next_paths = group.collect_vec();

        let mut is_leaf = false;
        for ref mut el in next_paths.iter_mut() {
            el.remove(0);
            if el.is_empty() {
                //removing last part means it's a leaf
                is_leaf = true;
            }
        }

        next_paths.retain(|el| !el.is_empty()); //remove empty paths

        if next_paths.is_empty() {
            next.insert(key.to_string(), NodeTree::IsLeaf);
        } else {
            next_paths.sort_by_key(|el| el[0].clone());
            let sub_tree = if !is_leaf { to_node_tree(next_paths) } else { NodeTree::IsLeaf };
            // let mut sub_tree = to_node_tree(next_paths);
            // sub_tree.is_leaf = is_leaf;
            next.insert(key.to_string(), sub_tree);
        }
    }
    NodeTree::new(next)
}
