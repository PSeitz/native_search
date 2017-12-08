#[cfg(test)]
mod tests {
    extern crate env_logger;

    #[allow(unused_imports)]
    use doc_loader;
    #[allow(unused_imports)]
    use util;
    #[allow(unused_imports)]
    use persistence;
    #[allow(unused_imports)]
    use util::normalize_text;
    #[allow(unused_imports)]
    use create;
    #[allow(unused_imports)]
    use search_field;
    use search;
    #[allow(unused_imports)]
    use serde_json;
    #[allow(unused_imports)]
    use serde_json::Value;
    use std::fs::File;
    use std::fs;
    use std::io::prelude::*;
    use trace;
    use fnv::FnvHashMap;
    use std::sync::RwLock;

    static TEST_DATA: &str = r#"[
        {
            "commonness": 123456,
            "ent_seq": "99999"
        },
        {
            "commonness": 20,
            "kanji": [
                { "text": "偉容", "commonness": 0},
                { "text": "威容","commonness": 5}
            ],
            "kana": [
                {
                    "text": "いよう",
                    "romaji": "Iyou",
                    "commonness": 5
                }
            ],
            "meanings": {
                "eng" : ["dignity", "majestic appearance", "will test"],
                "ger": ["majestätischer Anblick (m)", "majestätisches Aussehen (n)", "Majestät (f)"]
            },
            "ent_seq": "1587680"
        },
        {
            "commonness": 20,
            "kanji": [
                { "text": "意欲", "commonness": 40},
                { "text": "意慾", "commonness": 0}
            ],
            "kana": [
                {
                    "text": "いよく",
                    "romaji": "Iyoku",
                    "commonness": 40
                }
            ],
            "meanings": {
                "eng" : ["will", "desire", "urge", "having a long torso"],
                "ger": ["Wollen (n)", "Wille (m)", "Begeisterung (f)", "begeistern"]
            },
            "ent_seq": "1587690"
        },
        {
            "id": 1234566,
            "gender": "male",
            "birthDate": "1960-08-19",
            "address": [
                {
                    "line": [
                        "nuts strees"
                    ]
                }
            ],
            "commonness": 500,
            "kanji": [
                { "text": "意慾", "commonness": 20}
            ],
            "field1" : [{"text":"awesome", "rank":1}],
            "kana": [
                {
                    "text": "いよく"
                }
            ],
            "meanings": {
                "eng" : ["test1"],
                "ger": ["der test", "das ist ein guter Treffer"]
            },
            "ent_seq": "1587700"
        },
        {
            "id": 123456,
            "gender": "female",
            "birthDate": "1950-08-19",
            "address": [
                {
                    "line": [
                        "71955 Ilene Brook"
                    ]
                }
            ],
            "commonness": 551,
            "kanji": [
                {
                    "text": "何の",
                    "commonness": 526
                }
            ],
            "field1" : [{"text":"awesome"}, {"text":"nixhit"}],
            "kana": [
                {
                    "text": "どの",
                    "romaji": "Dono",
                    "commonness": 25
                }
            ],
            "meanings": {
                "ger": ["welch", "guter nicht Treffer", "alle meine Words"]
            },
            "ent_seq": "1920240"
        },
        {
            "pos": [
                "adj-i"
            ],
            "commonness": 1,
            "misc": [],
            "kanji": [
                {
                    "text": "柔らかい",
                    "commonness": 57
                }
            ],
            "kana": [
                {
                    "text": "やわらかい",
                    "romaji": "Yawarakai",
                    "commonness": 30
                }
            ],
            "meanings": {
                "ger": [
                    "(1) weich",
                    "stopword"
                ]
            },
            "ent_seq": "1605630"
        }
    ]"#;

    static TOKEN_VALUE: &str = r#"[
        {
            "text": "Begeisterung",
            "value": 20
        }
    ]"#;

    static TEST_FOLDER: &str = "mochaTest";
    lazy_static! {
        static ref PERSISTENCES: RwLock<FnvHashMap<String, persistence::Persistence>> = {
            RwLock::new(FnvHashMap::default())
        };
        static ref INDEX_CREATED: RwLock<bool> = RwLock::new(false);
    }


    #[test]
    fn test_paths() {
        let paths = util::get_steps_to_anchor("meanings.ger[]");
        println!("NAAA {:?}", paths);
    }

    #[test]
    #[ignore]
    fn test_binary_search() {
        let x = vec![1, 2, 3, 6, 7, 8];
        let u = x.binary_search(&4).unwrap_err();
        println!("{:?}", u);
        let value = match x.binary_search(&4) {
            Ok(value) => value,
            Err(value) => value,
        };
        println!("mjjaaa {}", value);
    }

    #[test]
    fn test_json_request() {
        warn!("can log from the test too");
        let requesto: search::Request = serde_json::from_str(r#"{"search":{"path":"asdf", "terms":[ "asdf"], "levenshtein_distance":1}}"#)
            .unwrap();
        println!("mjjaaa {:?}", requesto);
        assert_eq!(requesto.search.unwrap().levenshtein_distance, Some(1));
    }


    fn search_testo_to_doc(req: Value) -> Vec<search::DocWithHit> {
        search_testo_to_doco(req).expect("search error")
    }

    fn search_testo_to_doco(req: Value) -> Result<Vec<search::DocWithHit>, search::SearchError> {
        let persistences = PERSISTENCES.read().unwrap();
        let pers = persistences.get(&"default".to_string()).expect("Can't find loaded persistence");
        let requesto: search::Request = serde_json::from_str(&req.to_string()).expect("Can't parse json");
        let hits = search::search(requesto, pers)?;
        Ok(search::to_documents(pers, &hits.data))
    }

    describe! search_test {
        before_each {

            if let Ok(mut INDEX_CREATEDO) = INDEX_CREATED.write() {

                if !*INDEX_CREATEDO {
                    trace::enable_log();

                    // Start up a test.
                    let indices = r#"
                    [
                        { "boost":"commonness" , "options":{"boost_type":"int"}},
                        { "fulltext":"ent_seq" },
                        { "boost":"field1[].rank" , "options":{"boost_type":"int"}},
                        { "fulltext":"field1[].text" },
                        { "fulltext":"kanji[].text" },
                        { "fulltext":"meanings.ger[]", "options":{"tokenize":true, "stopwords": ["stopword"]} },
                        { "fulltext":"meanings.eng[]", "options":{"tokenize":true} },
                        { "fulltext":"address[].line[]", "options":{"tokenize":true} },
                        { "boost":"kanji[].commonness" , "options":{"boost_type":"int"}},
                        { "boost":"kana[].commonness", "options":{"boost_type":"int"} }
                    ]
                    "#;
                    // let indices = r#"
                    // [
                    //     { "fulltext":"address[].line[]", "options":{"tokenize":true} }
                    // ]
                    // "#;
                    println!("{:?}", create::create_indices(TEST_FOLDER, TEST_DATA, indices));

                    {
                        let mut pers = persistence::Persistence::load(TEST_FOLDER.to_string()).expect("Could not load persistence");
                        // let mut pers = persistence::Persistence::load(TEST_FOLDER.to_string()).expect("Could not load persistence");
                        let config = json!({
                            "path": "meanings.ger[]"
                        });
                        create::add_token_values_to_tokens(&mut pers, TOKEN_VALUE, &config.to_string()).expect("Could not add token values");

                    }

                    let mut persistences = PERSISTENCES.write().unwrap();
                    persistences.insert("default".to_string(), persistence::Persistence::load(TEST_FOLDER.to_string()).expect("could not load persistence"));

                    *INDEX_CREATEDO = true;
                }
            }
        }

        it "makes organizing tests easy" {
            let req = json!({
                "search": {
                    "terms":["majestätischer"],
                    "path": "meanings.ger[]",
                    "levenshtein_distance": 1,
                    "firstCharExactMatch": true
                }
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].doc["ent_seq"], "1587680");
        }

       it "deep structured objects" {
           // static TEST_DATA: &str = r#"[
           //     {
           //         "address": [
           //             {
           //                 "line": [ "line1" ]
           //             }
           //         ]
           //     },
           //     {
           //         "address": [
           //             {
           //                 "line": [ "line2" ]
           //             }
           //         ]
           //     }
           // ]"#;


           // let indices = r#"
           // [
           //     { "fulltext":"address[].line[]"}
           // ]
           // "#;
           // println!("{:?}", create::create_indices(TEST_FOLDER, TEST_DATA, indices));


           let req = json!({
               "search": {
                   "terms":["brook"],
                   "path": "address[].line[]",
                   "levenshtein_distance": 1
               }
           });

           let hits = search_testo_to_doc(req);
           assert_eq!(hits.len(), 1);
           assert_eq!(hits[0].doc["id"], 123456);
       }


        it "should search without firstCharExactMatch"{
            let req = json!({
                "search": {
                    "terms":["najestätischer"],
                    "path": "meanings.ger[]",
                    "levenshtein_distance": 1
                }
            });
            let hits = search_testo_to_doc(req);

            // println!("hits {:?}", hits);
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].doc["ent_seq"], "1587680");
        }

        it "should prefer exact matches to tokenmatches'"{

            let req = json!({
                "search": {
                    "terms":["will"],
                    "path": "meanings.eng[]",
                    "levenshtein_distance": 1
                }
            });
            let wa = search_testo_to_doc(req);
            // assert_eq!(wa.len(), 11);
            assert_eq!(wa[0].doc["meanings"]["eng"][0], "will");
        }

        it "should search word non tokenized'"{
            let req = json!({
                "search": {
                    "terms":["偉容"],
                    "path": "kanji[].text"
                }
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].doc["ent_seq"], "1587680");
        }

        it "should search on non subobject'"{
            let req = json!({
                "search": {
                    "terms":["1587690"],
                    "path": "ent_seq"
                }
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 1);
        }

        it "AND connect hits same field"{
            let req = json!({
                "and":[
                    {"search": {"terms":["aussehen"],       "path": "meanings.ger[]"}},
                    {"search": {"terms":["majestätisches"], "path": "meanings.ger[]"}}
                ]
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 1);
            assert_eq!(hits[0].doc["ent_seq"], "1587680");
        }

        it "AND connect hits different fields"{
            let req = json!({
                "and":[
                    {"search": {"terms":["majestät"], "path": "meanings.ger[]"}},
                    {"search": {"terms":["majestic"], "path": "meanings.eng[]"}}
                ]
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 1);
        }

        it "AND connect hits different fields - no hit"{
            let req = json!({
                "and":[
                    {"search": {
                        "terms":["majestät"],
                        "path": "meanings.ger[]"
                    }},
                    {"search": {
                        "terms":["urge"],
                        "path": "meanings.eng[]"
                    }}
                ]
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 0);
        }

        it "OR connect hits"{
            let req = json!({
                "or":[
                    {"search": {
                        "terms":["majestät"],
                        "path": "meanings.ger[]"
                    }},
                    {"search": {
                        "terms":["urge"],
                        "path": "meanings.eng[]"
                    }}
                ]
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 2);
        }

        it "should search and boost"{
            let req = json!({
                "search": {
                    "terms":["意慾"],
                    "path": "kanji[].text"
                },
                "boost" : [{
                    "path":"kanji[].commonness",
                    "boost_fun": "Log10",
                    "param": 1
                }]
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 2);
        }

        it "should search and double boost"{
            let req = json!({
                "search": {
                    "terms":["awesome"],
                    "path": "field1[].text"
                },
                "boost" : [{
                    "path":"commonness",
                    "boost_fun": "Log10",
                    "param": 1
                },
                {
                    "path":"field1[].rank",
                    "expression": "10 / $SCORE",
                    "skip_when_score" : [0]
                }]
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits.len(), 2);
        }

        it "should search and boost anchor"{
            let req = json!({
                "search": {
                    "terms":["意慾"],
                    "path": "kanji[].text",
                    "levenshtein_distance": 0,
                    "firstCharExactMatch":true
                },
                "boost" : [{
                    "path":"commonness",
                    "boost_fun": "Log10",
                    "param": 1
                }]
            });

            let hits = search_testo_to_doc(req);
            assert_eq!(hits[0].doc["commonness"], 500);
        }

        // it('should suggest', function() {
        //     return searchindex.suggest({path:'meanings.ger[]', term:'majes'}).then(res => {
        //         // console.log(JSON.stringify(res, null, 2))
        //         return Object.keys(res)
        //     })
        //     .should.eventually.have.length(5)
        // })


        it "should use search for suggest without sorting etc."{
            let req = json!({
                "terms":["majes"],
                "path": "meanings.ger[]",
                "levenshtein_distance": 0,
                "starts_with":true,
                "return_term":true
            });
            let requesto: search::RequestSearchPart = serde_json::from_str(&req.to_string()).expect("Can't parse json");
            let persistences = PERSISTENCES.read().unwrap();
            let mut pers = persistences.get(&"default".to_string()).unwrap();
            let results = search_field::get_hits_in_field(&mut pers, &requesto).unwrap();
            let mut all_terms = results.terms.values().collect::<Vec<&String>>();
            all_terms.sort();
            assert_eq!(all_terms, ["majestät", "majestätischer", "majestätischer anblick", "majestätisches", "majestätisches aussehen"]);
        }

        it "should load the text for ids"{
            let persistences = PERSISTENCES.read().unwrap();
            let mut pers = persistences.get(&"default".to_string()).unwrap();
            let mut faccess:persistence::FileSearch = pers.get_file_search("meanings.ger[].textindex");

            assert_eq!(faccess.get_text_for_id(0, pers.get_offsets("meanings.ger[].textindex").unwrap()), "alle" );
            assert_eq!(faccess.get_text_for_id(1, pers.get_offsets("meanings.ger[].textindex").unwrap()), "alle meine words" );
            assert_eq!(faccess.get_text_for_id(2, pers.get_offsets("meanings.ger[].textindex").unwrap()), "anblick" );

        }

        it "real suggest with score"{
            let req = json!({
                "terms":["majes"],
                "path": "meanings.ger[]",
                "levenshtein_distance": 0,
                "starts_with":true,
                "top":10,
                "skip":0
            });
            let requesto: search::RequestSearchPart = serde_json::from_str(&req.to_string()).expect("Can't parse json");
            let persistences = PERSISTENCES.read().unwrap();
            let mut pers = persistences.get(&"default".to_string()).unwrap();
            let results = search_field::suggest(&mut pers, &requesto).unwrap();
            assert_eq!(results.iter().map(|el| el.0.clone()).collect::<Vec<String>>(), ["majestät", "majestätischer", "majestätisches",
                                                                                        "majestätischer anblick", "majestätisches aussehen"]);
        }

        it "multi real suggest with score"{

            let req = json!({
                "suggest" : [
                    {"terms":["will"], "path": "meanings.ger[]", "levenshtein_distance": 0, "starts_with":true},
                    {"terms":["will"], "path": "meanings.eng[]", "levenshtein_distance": 0, "starts_with":true}
                ],
                "top":10,
                "skip":0
            });

            let requesto: search::Request = serde_json::from_str(&req.to_string()).expect("Can't parse json");
            let persistences = PERSISTENCES.read().unwrap();
            let mut pers = persistences.get(&"default".to_string()).unwrap();
            let results = search_field::suggest_multi(&mut pers, requesto).unwrap();
            assert_eq!(results.iter().map(|el| el.0.clone()).collect::<Vec<String>>(), ["will", "wille", "will test"]);
        }


        it "real suggest with boosting score of begeisterung and token value"{
            let req = json!({
                "terms":["begeist"],
                "path": "meanings.ger[]",
                "levenshtein_distance": 0,
                "starts_with":true,
                "token_value": {
                    "path":"meanings.ger[].textindex.tokenValues",
                    "boost_fun":"Log10",
                    "param": 1
                },
                "top":10,
                "skip":0
            });
            let requesto: search::RequestSearchPart = serde_json::from_str(&req.to_string()).expect("Can't parse json");
            let persistences = PERSISTENCES.read().unwrap();
            let mut pers = persistences.get(&"default".to_string()).unwrap();
            let results = search_field::suggest(&mut pers, &requesto).unwrap();
            assert_eq!(results.iter().map(|el| el.0.clone()).collect::<Vec<String>>(), ["begeisterung", "begeistern"]);
        }

        // it "should or connect the checks"{
        //     let req = json!({
        //         "search": {
        //             "terms":["having ]a long",
        //             "path": "meanings.eng[]",
        //             "levenshtein_distance": 1,
        //             "firstCharExactMatch":true,
        //             startsWith:true,
        //             operator:"some"
        //         }]
        //     });

        //     let hits = search_testo_to_doc(req);
        //     assert_eq!(hits.len(), 1);
        // }


        it "should rank exact matches pretty good"{
            let req = json!({
                "search": {
                    "terms":["weich"], // hits welche and weich
                    "path": "meanings.ger[]",
                    "levenshtein_distance": 1,
                    "firstCharExactMatch":true
                },
                "boost" : [{
                    "path":"commonness",
                    "boost_fun": "Log10",
                    "param": 1
                }]
            });

            let hits = search_testo_to_doc(req);
            println!("{:?}", hits);
            assert_eq!(hits[0].doc["meanings"]["ger"][0], "(1) weich");
        }

        it "OR connect hits, but boost one term"{
            let req = json!({
                "or":[
                    {"search": {"terms":["majestät"], "path": "meanings.ger[]", "boost": 2}},
                    {"search": {"terms":["urge"], "path": "meanings.eng[]"}}
                ]
            });

            let hits = search_testo_to_doc(req);
            println!("{:?}", hits);
            assert_eq!(hits.len(), 2);
            assert_eq!(hits[0].doc["meanings"]["ger"][0], "majestätischer Anblick (m)");
        }

        //MUTLI TERMS

        // { // multi terms attribute ALL
        //     let req = json!({
        //         "or":[{"search": {"terms":["alle","Words"], "path": "meanings.ger[]", "term_operator": "ALL"}} ]
        //     });

        //     let hits = search_test_to_doc(req, &mut pers);
        //     assert_eq!(hits[0].doc["meanings"]["ger"][2], "alle meine Words");
        // }

        // { // multi terms attribute ALL
        //     let req = json!({
        //         "or":[{"search": {"terms":["alle","Words", "TRIFFTNICHT"], "path": "meanings.ger[]", "term_operator": "ANY"}} ]
        //     });

        //     let hits = search_test_to_doc(req, &mut pers);
        //     assert_eq!(hits[0].doc["meanings"]["ger"][2], "alle meine Words");
        // }

        // { // terms
        //     let req = json!({
        //         "or":[
        //             {"search": {"terms":["guter","Treffer"], "path": "meanings.ger[]"}}
        //         ]
        //     });

        //     let hits = search_test_to_doc(req, &mut pers);
        //     println!("{:?}", hits);
        //     // assert_eq!(hits.as_ref().unwrap().len(), 2);
        //     assert_eq!(hits[0].doc["meanings"]["ger"][1], "das ist ein guter Treffer");
        // }

    }


    // fn load_test_data() -> &'static persistence::Persistence  {
    //     let persistences = PERSISTENCES.read().unwrap();
    //     persistences.get(&"default".to_string()).unwrap()
    // }

    // #[test]
    // fn checked_was_abgehst_22() {
    //     let small_test_json:&str = r#"[
    //         {
    //             "meanings": {
    //                 "eng" : ["dignity", "majestic appearance", "will test"],
    //                 "ger": ["majestätischer Anblick (m)", "stopword", "majestätisches Aussehen (n)", "Majestät (f)"]
    //             },
    //             "ent_seq": "1587680"
    //         }
    //     ]"#;

    //     let indices = r#"
    //     [
    //         { "fulltext":"meanings.ger[]", "options":{"tokenize":true, "stopwords": ["stopword"]} }
    //     ]
    //     "#;

    //     println!("{:?}", create::create_indices("rightTerms", small_test_json, indices));

    //     assert_eq!(normalize_text("Hello"), "Hello");

    //     let mut f = File::open("meanings.ger[]").unwrap();
    //     let mut s = String::new();
    //     f.read_to_string(&mut s).unwrap();

    //     let lines = s.lines().collect::<Vec<_>>();
    //     println!("{:?}", lines);
    //     let text = vec!["Anblick", "Aussehen", "Majestät", "majestätischer", "majestätischer Anblick", "majestätisches", "majestätisches Aussehen"];
    //     assert_eq!(lines, text);

    // }

    #[test]
    fn create_and_delete_file_in_subfolder() {
        fs::create_dir_all("subFolder1").unwrap();
        let some_terms = vec!["yep, yep"];
        File::create("subFolder1/test1").unwrap().write_all(some_terms.join("\n").as_bytes()).unwrap();
        assert_eq!("lines", "lines");
        println!("{:?}", fs::remove_dir_all("subFolder1"));
    }



}




// #[cfg(test)]
// mod test {
//     use create;
//     use serde_json;
//     use serde_json::Value;

//     #[test]
//     fn test_ewwwwwwwq() {

//         let opt: create::FulltextIndexOptions = serde_json::from_str(r#"{"tokenize":true, "stopwords": []}"#).unwrap();
//         // let opt = create::FulltextIndexOptions{
//         //     tokenize: true,
//         //     stopwords: vec![]
//         // };

//         let dat2 = r#" [{ "name": "John Doe", "age": 43 }, { "name": "Jaa", "age": 43 }] "#;
//         let data: Value = serde_json::from_str(dat2).unwrap();
//         let res = create::create_fulltext_index(&data, "name", opt);
//         println!("{:?}", res);
//         let deserialized: create::BoostIndexOptions = serde_json::from_str(r#"{"boost_type":"int"}"#).unwrap();

//         assert_eq!("Hello", "Hello");

//         let service: create::CreateIndex = serde_json::from_str(r#"{"boost_type":"int"}"#).unwrap();
//         println!("service: {:?}", service);

//     }
// }
