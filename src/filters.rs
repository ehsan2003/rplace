use futures::future;

use std::net::SocketAddr;

use std::net::IpAddr;

use warp::Filter;

pub(crate) fn ip_extractor() -> impl Filter<Extract = (IpAddr,), Error = warp::Rejection> + Clone {
    warp::filters::addr::remote()
        .and(warp::filters::header::optional("X-Forwarded-For"))
        .and_then(|addr: Option<SocketAddr>, forwarded_for: Option<IpAddr>| {
            match forwarded_for
                .iter()
                .chain(addr.map(|a| a.ip()).iter())
                .next()
                .copied()
            {
                Some(ip) => future::ok(ip),
                None => future::err(warp::reject()),
            }
        })
}
