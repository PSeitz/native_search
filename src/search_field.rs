
use str;
use persistence::Persistence;
use search::RequestSearchPart;
use search::SearchError;
use util::concat;
use std::cmp;
use fnv::FnvHashMap;

#[allow(unused_imports)]
use fst::{IntoStreamer, Levenshtein, Set, Map, MapBuilder};
use fst::automaton::*;

fn get_default_score(term1: &str, term2: &str) -> f32{
    return 2.0/(distance(term1, term2) as f32 + 0.2 )
}
fn get_default_score2(distance: u32) -> f32{
    return 2.0/(distance as f32 + 0.2 )
}

#[inline(always)]
fn get_text_lines<F>(persistence:&Persistence, options: &RequestSearchPart, mut fun: F) -> Result<(), SearchError>
where F: FnMut(&str, u32) {

    // let mut f = persistence.get_file_handle(&(options.path.to_string()+".fst"))?;
    // let mut buffer: Vec<u8> = Vec::new();
    // f.read_to_end(&mut buffer)?;
    // buffer.shrink_to_fit();
    // let map = try!(Map::from_bytes(buffer));

    // let map = persistence.get_fst(&options.path)?;

    let map = persistence.cache.fst.get(&options.path).expect("load fst no found");
    let lev = try!(Levenshtein::new(&options.term, options.levenshtein_distance.unwrap_or(0)));
    // let stream = map.search(lev).into_stream();
    let hits = if options.starts_with.unwrap_or(false) {
        let stream = map.search(lev.starts_with()).into_stream();
        try!(stream.into_str_vec())
    }else{
        let stream = map.search(lev).into_stream();
        try!(stream.into_str_vec())
    };
    // let hits = try!(stream.into_str_vec());
    // debug!("hitso {:?}", hits);

    for (term, id) in hits {
        fun(&term, id as u32 );
    }
    // if exact_search.is_some() {
    //     let mut faccess:persistence::FileSearch = persistence.get_file_search(&options.path);
    //     let result = faccess.binary_search(&exact_search.unwrap(), persistence)?;
    //     if result.1 != -1 {
    //         fun(&result.0, result.1 as u32 );
    //     }

    // }else if character.is_some() {
    //     debug!("Search CharOffset for: {:?}", character.unwrap());
    //     let char_offset_info_opt = persistence.get_create_char_offset_info(&options.path, character.unwrap())?;
    //     debug!("CharOffset: {:?}", char_offset_info_opt);
    //     if char_offset_info_opt.is_none() {
    //         return Ok(())
    //     }
    //     let char_offset_info = char_offset_info_opt.unwrap();
    //     let mut f = persistence.get_file_handle(&options.path)?;
    //     let mut buffer:Vec<u8> = Vec::with_capacity((char_offset_info.byte_range_end - char_offset_info.byte_range_start) as usize);
    //     unsafe { buffer.set_len(char_offset_info.byte_range_end as usize - char_offset_info.byte_range_start as usize); }

    //     f.seek(SeekFrom::Start(char_offset_info.byte_range_start as u64))?;
    //     f.read_exact(&mut buffer)?;
    //     // let s = unsafe {str::from_utf8_unchecked(&buffer)};
    //     let s = str::from_utf8(&buffer)?; // @Temporary  -> use unchecked if stable
    //     // trace!("Loaded Text: {}", s);
    //     let lines = s.lines();
    //     let mut pos = 0;
    //     for line in lines{
    //         fun(&line, char_offset_info.line_offset as u32 + pos );
    //         pos += 1;
    //     }
    //     debug!("Checked {:?} lines", pos);

    // }else{
    //     let mut f = persistence.get_file_handle(&options.path)?;
    //     let mut s = String::new();
    //     f.read_to_string(&mut s)?;
    //     let lines = s.lines();
    //     for (line_pos, line) in lines.enumerate(){
    //         fun(&line, line_pos as u32)
    //     }
    // }
    Ok(())
}

#[derive(Debug)]
pub struct SearchFieldResult {
    pub hits: Vec<(u32, f32)>,
    pub terms: FnvHashMap<u32, String>
}

pub fn get_hits_in_field(persistence:&Persistence, options: &RequestSearchPart) -> Result<SearchFieldResult, SearchError> {
    debug_time!("get_hits_in_field");
    // let mut hits:FnvHashMap<u32, f32> = FnvHashMap::default();
    let mut result = SearchFieldResult{hits: vec![], terms:FnvHashMap::default()};
    // let mut hits:Vec<(u32, f32)> = vec![];
    // let checks:Vec<Fn(&str) -> bool> = Vec::new();
    // options.first_char_exact_match = options.exact || options.levenshtein_distance == 0 || options.starts_with.is_some(); // TODO fix

    // if options.levenshtein_distance.unwrap_or(0) == 0 && !options.starts_with.unwrap_or(false) {
    //     options.exact = Some(true);
    // }

    // let term_chars = options.term.chars().collect::<Vec<char>>();
    // let start_char = if options.exact.unwrap_or(false) || options.levenshtein_distance.unwrap_or(0) == 0 || options.starts_with.unwrap_or(false) && term_chars.len() >= 2 {
    //     Some(term_chars[0].to_string() + &term_chars[1].to_string())
    // }
    // else if options.first_char_exact_match.unwrap_or(false) { Some(term_chars[0].to_string() )
    // }
    // else { None };
    // let start_char_val = start_char.as_ref().map(String::as_ref);

    trace!("Will Check distance {:?}", options.levenshtein_distance.unwrap_or(0) != 0);
    // trace!("Will Check exact {:?}", options.exact);
    trace!("Will Check starts_with {:?}", options.starts_with);
    {
        let teh_callback = |line: &str, line_pos: u32| {
            // trace!("Checking {} with {}", line, term);
            let distance = if options.levenshtein_distance.unwrap_or(0) != 0 { Some(distance(&options.term, line))} else { None };
            let mut score = if distance.is_some() {get_default_score2(distance.unwrap())} else {get_default_score(&options.term, line)};
            options.boost.map(|boost_val| score = score * boost_val); // @FixMe Move out of loop?
            debug!("Hit: {:?}\tid: {:?} score: {:?}", line, line_pos, score);
            // hits.insert(line_pos, score);
            result.hits.push((line_pos, score));
            if options.return_term.unwrap_or(false) {
                result.terms.insert(line_pos, line.to_string());
            }
        };
        // let exact_search = if options.exact.unwrap_or(false) {Some(options.term.to_string())} else {None};
        get_text_lines(persistence, options, teh_callback)?;
    }
    debug!("{:?} hits in textindex {:?}", result.hits.len(), &options.path);
    trace!("hits in textindex: {:?}", result.hits);
    resolve_token_hits(persistence, &options.path, &mut result);
    Ok(result)

}


pub fn resolve_token_hits(persistence:&Persistence, path:&str, result: &mut SearchFieldResult ){
    debug_time!("resolve_token_hits");

    let has_tokens = persistence.meta_data.fulltext_indices.get(path).map_or(false, |fulltext_info| fulltext_info.tokenize);
    debug!("has_tokens {:?} {:?}", path, has_tokens);
    if !has_tokens { return; }
    // var hrstart = process.hrtime()
    // let cache_lock = persistence::INDEX_64_CACHE.read().unwrap();
    // let text_offsets = persistence.cache.index_64.get(&concat(&path, ".offsets"))
    //     .expect(&format!("Could not find {:?} in index_64 cache", concat(&path, ".offsets")));

    // let key = (concat(&path, ".textindex.tokens.tokenValIds"), concat(&path, ".textindex.tokens.parentValId"));

    let token_kvdata = persistence.get_valueid_to_parent(&concat(&path, ".textindex.tokens"));

    // let token_kvdata = persistence.cache.index_id_to_parent.get(&key).expect(&format!("Could not find {:?} in index_id_to_parent cache", key));
    // let mut token_hits:FnvHashMap<u32, f32> = FnvHashMap::default();
    let mut token_hits:Vec<(u32, f32, u32)> = vec![];
    for &(value_id, score) in result.hits.iter() {
        // let parent_ids_for_token = token_kvdata.get_parent_val_ids(*value_id, &cache_lock);

        // let ref parent_ids_for_token_opt = token_kvdata.get(*value_id as usize);
        let ref parent_ids_for_token_opt = token_kvdata.get_values(value_id as u64);
        debug_time!("resolve_token_hits to map");
        parent_ids_for_token_opt.as_ref().map(|parent_ids_for_token|{
            if parent_ids_for_token.len() > 0 {
                token_hits.reserve(parent_ids_for_token.len());
                for token_parentval_id in parent_ids_for_token {
                    // let parent_text_length = text_offsets[1 + *token_parentval_id as usize] - text_offsets[*token_parentval_id as usize];
                    // let token_text_length  = text_offsets[1 + *value_id as usize] - text_offsets[*value_id as usize];
                    // let adjusted_score = 2.0/(parent_text_length as f32 - token_text_length as f32) + 0.2;

                    // let the_score = token_hits.entry(*token_parentval_id as u32) // @Temporary
                    //     .or_insert(*hits.get(token_parentval_id).unwrap_or(&0.0));
                    // *the_score += adjusted_score;
                    // token_hits.push((*token_parentval_id, adjusted_score));
                    token_hits.push((*token_parentval_id, score, value_id));
                }
            }
        });

        // let ref parent_ids_for_token = token_kvdata.get[*value_id as usize];
        // trace!("value_id {:?}", value_id);
        // trace!("parent_ids_for_token {:?}", parent_ids_for_token);
    }
    debug!("checked {:?}, got {:?} token hits", result.hits.iter().count(), token_hits.iter().count());
    {
        // println!("{:?}", token_hits);
        debug_time!("token_hits.sort_by");
        token_hits.sort_by(|a, b| a.0.cmp(&b.0)); // sort by parent id
    }
    debug_time!("extend token_results");
    // hits.extend(token_hits);
    trace!("token_hits in textindex: {:?}", token_hits);
    if token_hits.len() > 0 {
        result.hits.reserve(token_hits.len());
        let mut current_group_id = token_hits[0].0;
        let mut current_score = token_hits[0].1;
        for hit in token_hits {
            if hit.0 != current_group_id {
                result.hits.push((current_group_id, current_score));
                current_group_id = hit.0;
                current_score = hit.1;
            }else{
                current_score = f32::max(current_score, hit.1);
                // in group // @FixMe Alter Ranking
            }
            // hits.insert(hit.0, hit.1);
        }
        // hits.insert(current_group_id, current_score);
        result.hits.push((current_group_id, current_score));

    }
    trace!("hits with tokens: {:?}", result.hits);
    // for hit in hits.iter() {
    //     trace!("NEW HITS {:?}", hit);
    // }

}



fn distance(s1: &str, s2: &str) -> u32 {
    let len_s1 = s1.chars().count();

    let mut column: Vec<u32> = Vec::with_capacity(len_s1+1);
    unsafe { column.set_len(len_s1+1); }
    for x in 0..len_s1+1 {
        column[x] = x as u32;
    }

    for (x, current_char2) in s2.chars().enumerate() {
        column[0] = x as u32  + 1;
        let mut lastdiag = x as u32;
        for (y, current_char1) in s1.chars().enumerate() {
            if current_char1 != current_char2 { lastdiag+=1 }
            let olddiag = column[y+1];
            column[y+1] = cmp::min(column[y+1]+1, cmp::min(column[y]+1, lastdiag));
            lastdiag = olddiag;

        }
    }
    column[len_s1]

}
