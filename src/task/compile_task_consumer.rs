use std::{env, io};

use crate::{
    controller::tex::tex_controller::compile_tex_from_mq, model::project::compile_app_params::CompileAppParams, rest::client::cv_client::update_queue_status
};
use log::{error, warn};
use redis::Commands;
use redis::{
    streams::{StreamId, StreamKey, StreamReadOptions, StreamReadReply}
};
use redlock::{Lock, RedLock};
use rust_wheel::config::{
    app::app_conf_reader::get_app_config,
    cache::redis_util::{delete_stream_element, get_con},
};
use std::net::ToSocketAddrs;

pub async fn consume_redis_stream() {
    let mut con = get_con();
    let stream_key = get_app_config("cv.compile_stream_redis_key");
    let redis_conn_str = env::var("REDIS_URL").unwrap();
    let stream_id = "0";
    loop {
        let rl = RedLock::new(vec![redis_conn_str.as_str()]);
        let lock;
        loop {
            match rl.lock("mutex".as_bytes(), 1000) {
                Ok(Some(l)) => {
                    lock = l;
                    break;
                }
                Ok(None) => (),
                Err(e) => {
                    error!(
                        "consume_redis_stream Error communicating with redis: {}, conn str: {}",
                        e, redis_conn_str
                    );
                    check_dns(&redis_conn_str);
                    check_dns("infra-server-service.reddwarf-pro.svc.cluster.local");
                    check_dns("kubernetes.default");
                }
            }
        }
        let options = StreamReadOptions::default().count(1).block(1000).noack();
        let result = con.xread_options(
            &[stream_key.as_str()], 
            &[stream_id], 
            &options
        );
        if let Err(e) = result.as_ref() {
            error!("read stream failed: {}", e);
            break;
        }
        let stream_reply: StreamReadReply = result.unwrap();
        for sk in stream_reply.clone().keys {
            match sk.key.as_str() {
                "texhub-server:proj:s-comp-queue" => {
                    handle_proj_compile_stream(sk, &rl, &lock).await;
                }
                _ => {
                    error!("not implement");
                }
            }
        }
    }
}

fn check_dns(host: &str) {
    // try to parse dns
    let check_result: Result<Vec<std::net::SocketAddr>, io::Error> = (host, 0)
        .to_socket_addrs()
        .map(|addr_iter| addr_iter.collect());
    match check_result {
        Ok(addresses) => {
            warn!("parse dns success, host:{}", host);
            for addr in addresses {
                warn!("address:{}", addr);
            }
        }
        Err(e) => {
            error!("parse dns failed: {}, host: {}", e, host);
        }
    }
}

async fn handle_proj_compile_stream(sk: StreamKey, rl: &RedLock, lock: &Lock<'_>) {
    for stream_id in sk.clone().ids {
        handle_proj_compile_record(stream_id, rl, lock, &sk).await;
    }
}

async fn handle_proj_compile_record(
    stream_id: StreamId,
    rl: &RedLock,
    lock: &Lock<'_>,
    sk: &StreamKey,
) {
    let param = do_task(&stream_id);
    let u_result = update_queue_status(1, &param.qid, Some(-1)).await;
    if !u_result {
        // do not return when update failed
        // it will make the redis stream retry and go into a dead loop
        error!("update status failed: {},sk:{:?}", u_result, stream_id);
    }
    let del_result = delete_stream_element(sk.key.as_str(), stream_id.id.clone());
    if let Err(e) = del_result {
        error!("delete stream failed: {}, stream id: {:?}", e, &stream_id);
        rl.unlock(&lock);
        return;
    }
    rl.unlock(&lock);
    compile_tex_from_mq(param).await;
}

fn do_task(stream_id: &StreamId) -> CompileAppParams {
    let fp_value: &redis::Value = stream_id.map.get("file_path").unwrap();
    let file_path = extract_string_value(fp_value);
    if file_path.is_none() {
        error!("read the file path is None {:?}", stream_id);
    }
    let op_value: &redis::Value = stream_id.map.get("out_path").unwrap();
    let out_path = extract_string_value(op_value);
    let pi_value: &redis::Value = stream_id.map.get("project_id").unwrap();
    let project_id = extract_string_value(pi_value);
    let rt_value: &redis::Value = stream_id.map.get("req_time").unwrap();
    let req_time = extract_string_value(rt_value);
    let qid_value: &redis::Value = stream_id.map.get("qid").unwrap();
    let qid = extract_string_value(qid_value);
    let vn_value: &redis::Value = stream_id.map.get("version_no").unwrap();
    let version_no = extract_string_value(vn_value);
    let log_file_value: &redis::Value = stream_id.map.get("log_file_name").unwrap();
    let log_file_name = extract_string_value(log_file_value);
    let created_time_value: &redis::Value = stream_id.map.get("proj_created_time").unwrap();
    let proj_created_time = extract_string_value(created_time_value);
    let param: CompileAppParams = CompileAppParams {
        file_path: file_path.unwrap(),
        out_path: out_path.unwrap(),
        project_id: project_id.unwrap(),
        req_time: req_time.unwrap().parse().unwrap(),
        qid: qid.unwrap().parse().unwrap(),
        version_no: version_no.unwrap(),
        log_file_name: log_file_name.unwrap(),
        proj_created_time: proj_created_time.unwrap().parse().unwrap(),
    };
    return param;
}

fn extract_string_value(value: &redis::Value) -> Option<String> {
    if let redis::Value::
    BulkString(data) = value {
        let bytes: Vec<u8> = data.clone().to_vec();
        String::from_utf8(bytes).ok()
    } else {
        None
    }
}
