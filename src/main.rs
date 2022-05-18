use std::env::{var};
use anyhow::{anyhow,Result};
use futures::stream::{FuturesUnordered,StreamExt};
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
    disp:String,
    url:String,
    search:String,
    grad:bool
}


impl Department {
    pub fn new(disp:String,url:String,search:String,grad:bool)->Department {
        Department {
            disp,
            url,
            search,
            grad
        }
    }
    async fn download_all(&self)->Result<Vec<Course>> {
        let url=format!("https://catalog.gatech.edu/courses-{}/{}",if self.grad {"grad"}  else {"undergrad"},self.url);
        let main_page=reqwest::get(&url)
            .await?
            .text()
            .await?;

        let re_str=format!(r#"{}\s+(\d+)\.\s+(.*?)\.\s+\d+\s+Credit"#,&self.disp);
        println!("Course name regex: {}",re_str);
        let regex=regex::Regex::new(&re_str)?;
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
                        course_futures.push(self.get(id,name.as_str().to_string()));
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
    pub async fn get(&self,id:usize,name:String)->Result<Course> {
        let query=format!("georgia tech {} {} oscar",self.search,id);
        println!("{}",query);
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
        let resp=client.get("https://api.avesapi.com/search")
            .query(&[
                ("apikey",&val[..]),
                ("query",&query[..]),
                ("hl","en"),
                ("gl","US"),
                ("type","web")
            ])
            .send()
            .await?;

        let Resp {result:Res {organic_results:orgs}}=resp.json()
            .await?;
        let urls=orgs.iter().map(|x|x.url.clone()).collect::<String>();
        let res:Option<OrgRes>=orgs.into_iter().find(|org|{
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

                        let prereq_str=format!(r#"{}\s+(\d+)"#,self.disp);
                        let cs_re=regex::Regex::new(&prereq_str)?;
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
    let cs_dept=Department::new("MATH".to_string(),"math".to_string(),"math".to_string(),false);
    let courses=cs_dept.download_all().await?;
    let ser=serde_json::to_string(&(&courses,&cs_dept)).unwrap();

    let filename=format!("docs/courses.json");
    fs::write(&filename,&ser).expect("Couldn't write");

    println!("Done. {} courses found.",courses.len());
    Ok(())
}
