
#[allow(unused_imports)]
use std::io::{self, BufRead};
#[allow(unused_imports)]
use std::path::Path;
use std::cmp;

use std;
#[allow(unused_imports)]
use std::{str, f32, thread};
#[allow(unused_imports)]
use std::sync::mpsc::sync_channel;

#[allow(unused_imports)]
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::cmp::Ordering;

#[allow(unused_imports)]
use fnv::FnvHashMap;

use serde_json;
#[allow(unused_imports)]
use std::time::Duration;

use search_field;
use persistence::Persistence;
use doc_loader::DocLoader;
use util;


#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct Request {
    pub or : Option<Vec<Request>>,
    pub and : Option<Vec<Request>>,
    pub search: Option<RequestSearchPart>,
    pub boost: Option<Vec<RequestBoostPart>>
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct RequestSearchPart {
    pub path: String,
    pub term: String,
    pub levenshtein_distance: Option<u32>,
    pub starts_with: Option<String>,
    pub exact: Option<bool>,
    pub first_char_exact_match: Option<bool>
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct RequestBoostPart {
    pub path: String,
    pub boost_fun: BoostFunction,
    pub param: Option<f32>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum BoostFunction {
    Log10,
}

impl Default for BoostFunction {
    fn default() -> BoostFunction { BoostFunction::Log10 }
}

// pub enum CheckOperators {
//     All,
//     One
// }
// impl Default for CheckOperators {
//     fn default() -> CheckOperators { CheckOperators::All }
// }


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub struct Hit {
    pub id: u32,
    pub score: f32
}

fn hits_to_array(hits:FnvHashMap<u32, f32>) -> Vec<Hit> {
    debugTime!("hits_to_array");
    let mut res:Vec<Hit> = hits.iter().map(|(id, score)| Hit{id:*id, score:*score}).collect();
    res.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap_or(Ordering::Equal)); // Add sort by id
    res
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DocWithHit {
    pub doc: serde_json::Value,
    pub hit: Hit
}


impl std::fmt::Display for DocWithHit {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        write!(f, "\n{}\t{}", self.hit.id, self.hit.score )?;
        write!(f, "\n{}", serde_json::to_string_pretty(&self.doc).unwrap() )?;
        Ok(())
    }
}

pub fn to_documents(persistence:&mut Persistence, hits: &Vec<Hit>) -> Vec<DocWithHit> {
    DocLoader::load(persistence);
    hits.iter().map(|ref hit| {
        let doc = DocLoader::get_doc(persistence, hit.id as usize).unwrap();
        DocWithHit{doc:serde_json::from_str(&doc).unwrap(), hit:*hit.clone()}
    }).collect::<Vec<_>>()
}
pub fn search(request: Request, skip:usize, mut top:usize, persistence:&Persistence) -> Result<Vec<Hit>, SearchError>{
    infoTime!("search");
    let res = search_unrolled(&persistence, request)?;
    // println!("{:?}", res);
    // let res = hits_to_array_iter(res.iter());
    let res = hits_to_array(res);
    top = cmp::min(top + skip, res.len());
    Ok(res[skip..top].to_vec())
}

fn get_shortest_result<T: std::iter::ExactSizeIterator>(results: &Vec<T>) -> usize {
    let mut shortest = (0, std::u64::MAX);
    for (index, res) in results.iter().enumerate(){
        if (res.len() as u64) < shortest.1 {
            shortest =  (index, res.len() as u64);
        }
    }
    shortest.0
}

pub fn search_unrolled(persistence:&Persistence, request: Request) -> Result<FnvHashMap<u32, f32>, SearchError>{
    infoTime!("search_unrolled");
    if request.or.is_some() {
        Ok(request.or.unwrap().iter()
            .fold(FnvHashMap::default(), |mut acc, x| -> FnvHashMap<u32, f32> {
                acc.extend(&search_unrolled(persistence, x.clone()).unwrap());
                acc
            }))
    }else if request.and.is_some(){
        let ands = request.and.unwrap();
        let mut and_results:Vec<FnvHashMap<u32, f32>> = ands.iter().map(|x| search_unrolled(persistence, x.clone()).unwrap()).collect(); // @Hack  unwrap forward errors

        debugTime!("and algorithm");
        let mut all_results:FnvHashMap<u32, f32> = FnvHashMap::default();
        let index_shortest = get_shortest_result(&and_results.iter().map(|el| el.iter()).collect());

        let shortest_result = and_results.swap_remove(index_shortest);
        for (k, v) in shortest_result {
            if and_results.iter().all(|ref x| x.contains_key(&k)){
                all_results.insert(k, v);
            }
        }
        // for res in &and_results {
        //     all_results.extend(res); // merge all results
        // }

        Ok(all_results)
    }else if request.search.is_some(){
        Ok(search_raw(persistence, request.search.unwrap(), request.boost)?)
    }else{
        Ok(FnvHashMap::default())
    }
}

#[allow(dead_code)]
fn add_boost(persistence: &Persistence, boost: &RequestBoostPart, hits: &mut FnvHashMap<u32, f32>) -> Result<(), SearchError> {
    let key = util::boost_path(&boost.path);
    let boostkv_store = persistence.index_id_to_parent.get(&key).expect(&format!("Could not find {:?} in index_id_to_parent cache", key));
    let boost_param = boost.param.unwrap_or(0.0);
    for (value_id, score) in hits.iter_mut() {
        let ref values = boostkv_store[*value_id as usize];
        if values.len() > 0 {
            let boost_value = values[0]; // @Temporary // @Hack this should not be an array for this case
            match boost.boost_fun {
                BoostFunction::Log10 => {
                    *score += ( boost_value as f32 + boost_param).log10(); // @Temporary // @Hack // @Cleanup // @FixMe
                }
            }
        }
        // if let Some(boost_value) = boostkv_store.get_value(*value_id) {
        //     match boost.boost_fun {
        //         BoostFunction::Log10 => {
        //             *score += (boost_value  as f32 + boost_param).log10(); // @Temporary // @Hack // @Cleanup // @FixMe
        //         }
        //     }
        // }
    }
    Ok(())
}


#[derive(Debug)]
pub enum SearchError{
    Io(io::Error),
    MetaData(serde_json::Error),
    Utf8Error(std::str::Utf8Error)
}
// Automatic Conversion
impl From<io::Error>            for SearchError {fn from(err: io::Error) -> SearchError {SearchError::Io(err) } }
impl From<serde_json::Error>    for SearchError {fn from(err: serde_json::Error) -> SearchError {SearchError::MetaData(err) } }
impl From<std::str::Utf8Error>  for SearchError {fn from(err: std::str::Utf8Error) -> SearchError {SearchError::Utf8Error(err) } }

fn check_apply_boost(persistence:&Persistence, boost: &RequestBoostPart, path_name:&str, hits: &mut FnvHashMap<u32, f32>) -> bool {
    let will_apply_boost = boost.path.starts_with(path_name);
    if will_apply_boost{
        add_boost(persistence, boost, hits);
    }
    will_apply_boost
}

pub fn search_raw(persistence:&Persistence, mut request: RequestSearchPart, mut boost: Option<Vec<RequestBoostPart>>) -> Result<FnvHashMap<u32, f32>, SearchError> {
    let term = util::normalize_text(&request.term);
    infoTime!("search and join to anchor");
    let mut hits = search_field::get_hits_in_field(persistence, &mut request, &term)?;
    
    if hits.len() == 0 {return Ok(hits)};
    let mut next_level_hits:FnvHashMap<u32, f32> = FnvHashMap::default();
    // let mut next_level_hits:Vec<(u32, f32)> = vec![];

    let paths = util::get_steps_to_anchor(&request.path);
    info!("Joining {:?} hits from {:?} for {:?}", hits.len(), paths, term);
    for i in (0..paths.len()).rev() {
        let is_text_index = i == (paths.len() -1);
        let path_name = util::get_path_name(&paths[i], is_text_index);

        if boost.is_some() {
            boost.as_mut().unwrap().retain(|boost| check_apply_boost(persistence, boost, &path_name,&mut hits));
        }

        let key = util::concat_tuple(&path_name, ".valueIdToParent.valIds", ".valueIdToParent.mainIds");
        debugTime!("Joining to anchor");
        let kv_store = persistence.index_id_to_parent.get(&key).expect(&format!("Could not find {:?} in index_id_to_parent cache", key));
        debugTime!("Adding all values");
        next_level_hits.reserve(hits.len());
        for (value_id, score) in hits.iter() {
            // kv_store.add_values(*value_id, &cache_lock, *score, &mut next_level_hits);
            let ref values = kv_store[*value_id as usize];
            next_level_hits.reserve(values.len());
            trace!("value_id: {:?} values: {:?} ", value_id, values);
            for parent_val_id in values {    // @Temporary
                match next_level_hits.entry(*parent_val_id as u32) {
                    Vacant(entry) => { entry.insert(*score); },
                    Occupied(entry) => {
                        if *entry.get() < *score {
                            *entry.into_mut() = score.max(*entry.get()) + 0.1;
                        }
                    },
                }
            }

            // for parent_val_id in values {    // @Temporary
            //     next_level_hits.place_back() <- (parent_val_id, *score);
            //     // next_level_hits.push((parent_val_id, *score));
            // }

            // for parent_val_id in values {
            //     let hit = next_level_hits.get(parent_val_id as u64);
            //     if  hit.map_or(true, |el| el == f32::NEG_INFINITY) {
            //         next_level_hits.insert(parent_val_id as u64, score);
            //     }else{
            //         next_level_hits.insert(parent_val_id as u64, score);
            //     }
            // }
        }

        trace!("next_level_hits: {:?}", next_level_hits);
        debug!("{:?} hits in next_level_hits {:?}", next_level_hits.len(), &key.1);

        // debugTime!("sort and dedup");
        // next_level_hits.sort_by(|a, b| a.0.cmp(&b.0));
        // next_level_hits.dedup_by_key(|i| i.0);
        // hits.clear();
        // debugTime!("insert to next level");
        // hits.reserve(next_level_hits.len());
        // for el in &next_level_hits {
        //     hits.insert(el.0, el.1);
        // }
        // next_level_hits.clear();

        // hits.extend(next_level_hits.iter());
        hits = next_level_hits;
        next_level_hits = FnvHashMap::default();
    }

    Ok(hits)
}


// pub fn test_levenshtein(term:&str, max_distance:u32) -> Result<(Vec<String>), io::Error> {

//     use std::time::SystemTime;

//     let mut f = try!(File::open("de_full_2.txt"));
//     let mut s = String::new();
//     try!(f.read_to_string(&mut s));

//     let now = SystemTime::now();

//     let lines = s.lines();
//     let mut hits = vec![];
//     for line in lines{
//         let distance = distance(term, line);
//         if distance < max_distance {
//             hits.push(line.to_string())
//         }
//     }

//     let ms = match now.elapsed() {
//         Ok(elapsed) => {(elapsed.as_secs() as f64) * 1_000.0 + (elapsed.subsec_nanos() as f64 / 1000_000.0)}
//         Err(_e) => {-1.0}
//     };

//     let lines_checked = s.lines().count() as f64;
//     println!("levenshtein ms: {}", ms);
//     println!("Lines : {}", lines_checked );
//     let ms_per_1000 = ((ms as f64) / lines_checked) * 1000.0;
//     println!("ms per 1000 lookups: {}", ms_per_1000);
//     Ok((hits))

// }

