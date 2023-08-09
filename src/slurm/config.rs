use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

#[derive(Clone, Debug)]
pub struct Cloud {
    pub username: String,
    pub password: String,
    pub ssh_url: String,
    pub ssh_pri_key: String,
    pub work_path: String,
    pub info: String,
}

impl Cloud {
    pub fn user(&self) -> String {
        let username = &self.username;
        let user = if username.contains("@") {
            username.split("@").collect::<Vec<&str>>()[0]
        } else {
            username
        };

        user.to_string()
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub clouds: HashMap<String, Cloud>,
}

const CLOUD_BX1961: &'static str = "bx_scz1961";
const CLOUD_SGWZ: &'static str = "sg_aixplorerbio_wz";

impl Config {
    pub fn get_instance() -> Arc<Mutex<Config>> {
        static mut CONFIG: Option<Arc<Mutex<Config>>> = None;

        unsafe {
            // Rust中使用可变静态变量都是unsafe的
            CONFIG
                .get_or_insert_with(|| {
                    let mut map = HashMap::new();
                    map.insert(
                        CLOUD_SGWZ.to_string(),
                        Cloud {
                            username: "aixplorerbio_wz".to_owned(),
                            password: "".to_owned(),
                            ssh_url: "xh5.hpccube.com:65061".to_owned(),
                            ssh_pri_key: "/mnt/archive/ssh/id_rsa_sg_aixplorerbio_wz.txt"
                                .to_owned(),
                            work_path: "/work/home/aixplorerbio_wz/auto-work".to_owned(),
                            info: "曙光云".to_owned(),
                        },
                    );
                    map.insert(
                        CLOUD_BX1961.to_string(),
                        Cloud {
                            username: "scz1961@NC-N30".to_owned(),
                            password: "Ai123456".to_owned(),
                            ssh_url: "ssh.cn-zhongwei-1.paracloud.com:22".to_owned(),
                            ssh_pri_key: "/mnt/archive/ssh/id_rsa-scz1961".to_owned(),
                            work_path: "/HOME/scz1961/run/auto-work".to_owned(),
                            info: "并行云".to_owned(),
                        },
                    );

                    // 初始化单例对象的代码
                    Arc::new(Mutex::new(Config { clouds: map }))
                })
                .clone()
        }
    }

    pub fn check_cloud_name(key: String) -> bool {
        let clouds = Config::get_instance().lock().unwrap().clouds.clone();
        clouds.contains_key(&key)
    }

    pub fn cloud(name: String) -> Cloud {
        let clouds = Config::get_instance().lock().unwrap().clouds.clone();
        clouds.get(&name).unwrap().clone()
    }

    pub fn clouds() -> Vec<Cloud> {
        let clouds = Config::get_instance().lock().unwrap().clouds.clone();
        clouds
            .values()
            .into_iter()
            .map(|f| f.clone())
            .collect::<Vec<Cloud>>()
    }
}
