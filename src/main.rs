use std::env::{var};
use std::collections::HashMap;
use anyhow::{anyhow,Result};
use futures::stream::{FuturesUnordered,StreamExt};
use async_std::task;
use serde::{Serialize,Deserialize};
use std::fs;
use dotenv;

#[derive(Serialize,Deserialize,Debug)]
struct Course {
    id:usize,
    name:String,
    desc:String,
    prereqs:Vec<usize>
}

#[derive(Serialize,Deserialize,Debug)]
struct OrgRes {
    description:String,
    url:String,
    title:String
}

#[derive(Serialize,Deserialize,Debug)]
struct Res {
    organic_results:Vec<OrgRes>
}

#[derive(Serialize,Deserialize,Debug)]
struct Resp {
    result:Res
}

#[derive(Serialize,Deserialize,Debug)]
struct Department {
    courses:HashMap<usize,Course>
}

impl Department {
    pub fn new()->Department {
        Department {
            courses:HashMap::new()
        }
    }
    async fn download_all()->Result<Vec<Course>> {
        let main_page=reqwest::get("https://catalog.gatech.edu/courses-undergrad/cs/")
            .await?
            .text()
            .await?;

        let regex=regex::Regex::new(r#"CS\s+(\d+)\.\s+(.*?)\.\s+\d+\s+Credit"#)?;
        let mut course_futures=FuturesUnordered::new();
        let iter=regex
            .captures_iter(&main_page);
        for cap in iter{
            let id=cap.get(1);
            let name=cap.get(2);
            if let Some(id)=id {
                let id:Result<usize,_>=id.as_str().to_string().parse();
                if let Ok(id)=id {
                    if let Some(name)=name {
                        course_futures.push(Department::get(id,name.as_str().to_string()));
                    }
                }
            }
        }


        let mut courses:Vec<Course>=vec![];

        while let Some(course)=course_futures.next().await{
            if let Ok(course)=course {
                courses.push(course);
            } else if let Err(err)=course{
                println!("{:?}",err);
            }
        }
        /*
        while let Some(course)=course_futures.next().await {
            println!("Course done");
            courses.push(course?);
        }
        */

        Ok(courses)
    }
    pub async fn get(id:usize,name:String)->Result<Course> {
        let query=format!("georgia tech CS {} oscar",id);
        let val=var("KEY").expect("No Aves API key found.");
        /*
        let (key,val)=vars()
            .find(|(key,val)| key=="KEY")
            .expect("No Aves API key found. Set KEY in .env.");
        */
        /*
            .map_or(
                Err(anyhow!("No Aves API key found. Set KEY in .env.")),
                |(key,val)|Ok(val)
            )?;
        */
        let client=reqwest::Client::new();
        let Resp {result:Res {organic_results:orgs}}=client.get("https://api.avesapi.com/search")
            .query(&[
                ("apikey",&val[..]),
                ("query",&query[..]),
                ("hl","en"),
                ("gl","US"),
                ("type","web")
            ])
            .send()
            .await?
            .json()
            .await?;
        let urls=orgs.iter().map(|x|x.url.clone()).collect::<String>();
        let res:Option<OrgRes>=orgs.into_iter().find(|org|{
            //&org.description[..7]==format!("CS {}",id) &&
            &org.url[..25]=="https://oscar.gatech.edu/"
        });
        match res {
            Some(res)=>{

                let html=reqwest::get(res.url)
                    .await?
                    .text()
                    .await?;



                let prereq_pos=html.find("Prerequisites:");
                let prereqs=match prereq_pos {
                    Some(pos)=>{
                        let substr=&html[pos..];

                        let cs_re=regex::Regex::new(r#"CS\s+(\d+)"#)?;
                        cs_re
                            .captures_iter(substr)
                            .flat_map(|cap|{
                                let num=cap.get(1);
                                if let Some(num)=num {
                                    if let Ok(num)=num.as_str().to_string().parse::<usize>() {
                                        return Some(num)
                                    }
                                }
                                None
                            })
                            .collect()
                    }
                    None=>{
                        vec![]
                    }
                };

                let desc_re=regex::Regex::new(r#"<TD CLASS="ntdefault">\n(.*?)\n"#)?;
                let captures=desc_re.captures(&html);
                let description=captures.map(|captures|{
                    captures.get(1).map(|cap|{
                        cap.as_str().to_string()
                    }).unwrap_or_else(|| String::from(""))
                }).unwrap_or_else(|| String::from(""));

                Ok(Course {
                    id,
                    name,
                    desc:description,
                    prereqs
                })
            }
            None=>Err(anyhow!("No matching results. ID: {}, URLS: {:?}",id,urls))
        }
    }
}

#[tokio::main]
async fn main()->Result<()> {
dotenv::dotenv().ok();
    let courses=Department::download_all().await?;
    let ser=serde_json::to_string(&courses).unwrap();
    //println!("{}",ser);
    fs::write("out/courses.json",&ser).expect("Couldn't write");
    println!("Done. {} courses found.",courses.len());
    Ok(())
}
