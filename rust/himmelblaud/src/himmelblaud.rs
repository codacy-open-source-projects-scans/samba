/*
   Unix SMB/CIFS implementation.

   Himmelblau daemon

   Copyright (C) David Mulder 2024

   This program is free software; you can redistribute it and/or modify
   it under the terms of the GNU General Public License as published by
   the Free Software Foundation; either version 3 of the License, or
   (at your option) any later version.

   This program is distributed in the hope that it will be useful,
   but WITHOUT ANY WARRANTY; without even the implied warranty of
   MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
   GNU General Public License for more details.

   You should have received a copy of the GNU General Public License
   along with this program.  If not, see <http://www.gnu.org/licenses/>.
*/
use crate::cache::{GroupCache, PrivateCache, UidCache, UserCache};
#[cfg(not(test))]
use crate::himmelblaud::himmelblaud_pam_auth::AuthSession;
use bytes::{BufMut, BytesMut};
use dbg::{DBG_DEBUG, DBG_ERR, DBG_WARNING};
use futures::{SinkExt, StreamExt};
use himmelblau::graph::Graph;
use himmelblau::BrokerClientApplication;
use idmap::Idmap;
use kanidm_hsm_crypto::{BoxedDynTpm, MachineKey};
use param::LoadParm;
use sock::{PamAuthResponse, Request, Response};
use std::error::Error;
use std::io;
use std::io::{Error as IoError, ErrorKind};
use std::sync::Arc;
use tokio::net::UnixStream;
use tokio::sync::Mutex;
use tokio_util::codec::{Decoder, Encoder, Framed};

#[cfg(not(test))]
pub(crate) struct Resolver {
    realm: String,
    tenant_id: String,
    lp: LoadParm,
    idmap: Idmap,
    graph: Graph,
    pcache: PrivateCache,
    user_cache: UserCache,
    uid_cache: UidCache,
    group_cache: GroupCache,
    hsm: Mutex<BoxedDynTpm>,
    machine_key: MachineKey,
    client: Arc<Mutex<BrokerClientApplication>>,
}

#[cfg(not(test))]
impl Resolver {
    pub(crate) fn new(
        realm: &str,
        tenant_id: &str,
        lp: LoadParm,
        idmap: Idmap,
        graph: Graph,
        pcache: PrivateCache,
        user_cache: UserCache,
        uid_cache: UidCache,
        group_cache: GroupCache,
        hsm: BoxedDynTpm,
        machine_key: MachineKey,
        client: BrokerClientApplication,
    ) -> Self {
        Resolver {
            realm: realm.to_string(),
            tenant_id: tenant_id.to_string(),
            lp,
            idmap,
            graph,
            pcache,
            user_cache,
            uid_cache,
            group_cache,
            hsm: Mutex::new(hsm),
            machine_key,
            client: Arc::new(Mutex::new(client)),
        }
    }
}

// The test environment is unable to communicate with Entra ID, therefore
// we alter the resolver to only test the cache interactions.

#[cfg(test)]
pub(crate) struct Resolver {
    realm: String,
    tenant_id: String,
    lp: LoadParm,
    idmap: Idmap,
    pcache: PrivateCache,
    user_cache: UserCache,
    uid_cache: UidCache,
    group_cache: GroupCache,
}

#[cfg(test)]
impl Resolver {
    pub(crate) fn new(
        realm: &str,
        tenant_id: &str,
        lp: LoadParm,
        idmap: Idmap,
        pcache: PrivateCache,
        user_cache: UserCache,
        uid_cache: UidCache,
        group_cache: GroupCache,
    ) -> Self {
        Resolver {
            realm: realm.to_string(),
            tenant_id: tenant_id.to_string(),
            lp,
            idmap,
            pcache,
            user_cache,
            uid_cache,
            group_cache,
        }
    }
}

struct ClientCodec;

impl Decoder for ClientCodec {
    type Error = io::Error;
    type Item = Request;

    fn decode(
        &mut self,
        src: &mut BytesMut,
    ) -> Result<Option<Self::Item>, Self::Error> {
        match serde_json::from_slice::<Request>(src) {
            Ok(msg) => {
                src.clear();
                Ok(Some(msg))
            }
            _ => Ok(None),
        }
    }
}

impl Encoder<Response> for ClientCodec {
    type Error = io::Error;

    fn encode(
        &mut self,
        msg: Response,
        dst: &mut BytesMut,
    ) -> Result<(), Self::Error> {
        DBG_DEBUG!("Attempting to send response -> {:?} ...", msg);
        let data = serde_json::to_vec(&msg).map_err(|e| {
            DBG_ERR!("socket encoding error -> {:?}", e);
            io::Error::new(ErrorKind::Other, "JSON encode error")
        })?;
        dst.put(data.as_slice());
        Ok(())
    }
}

impl ClientCodec {
    fn new() -> Self {
        ClientCodec
    }
}

pub(crate) async fn handle_client(
    stream: UnixStream,
    resolver: Arc<Mutex<Resolver>>,
) -> Result<(), Box<dyn Error>> {
    DBG_DEBUG!("Accepted connection");

    let Ok(_ucred) = stream.peer_cred() else {
        return Err(Box::new(IoError::new(
            ErrorKind::Other,
            "Unable to verify peer credentials.",
        )));
    };

    let mut reqs = Framed::new(stream, ClientCodec::new());
    #[cfg(not(test))]
    let mut pam_auth_session_state = None;

    while let Some(Ok(req)) = reqs.next().await {
        let mut resolver = resolver.lock().await;
        let resp = match req {
            #[cfg(not(test))]
            Request::PamAuthenticateInit(account_id) => {
                DBG_DEBUG!("pam authenticate init");

                match &pam_auth_session_state {
                    Some(_) => {
                        DBG_WARNING!(
                            "Attempt to init \
                                                    auth session while current \
                                                    session is active"
                        );
                        pam_auth_session_state = None;
                        Response::Error
                    }
                    None => {
                        let (auth_session, resp) =
                            resolver.pam_auth_init(&account_id)?;
                        pam_auth_session_state = Some(auth_session);
                        resp
                    }
                }
            }
            #[cfg(not(test))]
            Request::PamAuthenticateStep(pam_next_req) => {
                DBG_DEBUG!("pam authenticate step");
                match &mut pam_auth_session_state {
                    Some(AuthSession::InProgress {
                        account_id,
                        cred_handler,
                    }) => {
                        let resp = resolver
                            .pam_auth_step(
                                account_id,
                                cred_handler,
                                pam_next_req,
                            )
                            .await?;
                        match resp {
                            Response::PamAuthStepResponse(
                                PamAuthResponse::Success,
                            ) => {
                                pam_auth_session_state =
                                    Some(AuthSession::Success);
                            }
                            Response::PamAuthStepResponse(
                                PamAuthResponse::Denied,
                            ) => {
                                pam_auth_session_state =
                                    Some(AuthSession::Denied);
                            }
                            _ => {}
                        }
                        resp
                    }
                    _ => {
                        DBG_WARNING!(
                            "Attempt to \
                                                    continue auth session \
                                                    while current session is \
                                                    inactive"
                        );
                        Response::Error
                    }
                }
            }
            Request::NssAccounts => resolver.getpwent().await?,
            Request::NssAccountByName(account_id) => {
                resolver.getpwnam(&account_id).await?
            }
            Request::NssAccountByUid(uid) => resolver.getpwuid(uid).await?,
            Request::NssGroups => resolver.getgrent().await?,
            Request::NssGroupByName(grp_id) => {
                resolver.getgrnam(&grp_id).await?
            }
            Request::NssGroupByGid(gid) => resolver.getgrgid(gid).await?,
            #[cfg(not(test))]
            Request::PamAccountAllowed(account_id) => {
                resolver.pam_acct_mgmt(&account_id).await?
            }
            Request::PamAccountBeginSession(_account_id) => Response::Success,
            #[cfg(test)]
            _ => Response::Error,
        };
        reqs.send(resp).await?;
        reqs.flush().await?;
        DBG_DEBUG!("flushed response!");
    }

    DBG_DEBUG!("Disconnecting client ...");
    Ok(())
}

mod himmelblaud_getgrent;
mod himmelblaud_getgrgid;
mod himmelblaud_getgrnam;
mod himmelblaud_getpwent;
mod himmelblaud_getpwnam;
mod himmelblaud_getpwuid;
#[cfg(not(test))]
mod himmelblaud_pam_acct_mgmt;
#[cfg(not(test))]
mod himmelblaud_pam_auth;
