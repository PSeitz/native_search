


#[cfg(test)]
mod bencho {
    use serde_json;
    use ForEachOpt;
    use IDHolder;
    use for_each_element;

    use test::Bencher;

    #[bench]
    fn name(b: &mut Bencher) {

        let long_string :Vec<serde_json::Value> = (0..50000).map(|_|
            json!({
                "a": 1,
                "more": ["ok", "nice"],
                "objects": [{
                    "stuff": "yii"
                },{
                    "stuff": "yaa"
                }]
            })
        ).collect();

        let mut opt = ForEachOpt {};
        let mut id_holder = IDHolder::new();

        let data = json!(long_string);

        b.iter(|| {

            // let texts = vec![];
            // texts.reserve(5000);
            let mut cb_text = |_value: &str, _path: &str, _parent_val_id: u32| {
                // println!("TEXT: path {} value {} parent_val_id {}",path, value, parent_val_id);
            };
            let mut callback_ids = |_path: &str, _val_id: u32, _parent_val_id: u32| {
                // println!("IDS: path {} val_id {} parent_val_id {}",path, val_id, parent_val_id);
            };


            for_each_element(&data, &mut id_holder, &mut opt, &mut cb_text, &mut callback_ids);
        })
    }



}