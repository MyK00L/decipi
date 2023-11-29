#![allow(dead_code)]

#[cfg(all(feature = "server", feature = "client"))]
compile_error!(
    "Features \"server\" and \"client\" are mutually exclusive and cannot be enabled together"
);

mod message;
mod socket;

pub use message::*;
use rand::{thread_rng, Rng};
use scc::HashMap;
#[cfg(feature="server")]
use scc::HashSet;
use socket::*;
use std::time::{Duration, SystemTime};
use tokio::task::AbortHandle;
use tokio::time::sleep;
use tokio::task;
#[cfg(feature="server")]
use tokio::join;
use tracing::*;

#[cfg(feature = "client")]
/// messages delivered to a client
pub enum RecvMessage {
    Queue(QueueMessage),
    File(FileMessage),
    Request(RequestMessage),
    EncKey(EncKeyInfo),
}
#[cfg(feature = "client")]
/// messages sent from a client
pub enum SendMessage {
    File(FileMessage),
    Request(RequestMessage),
    Submission(SubmissionMessage),
    Question(QuestionMessage),
}

#[cfg(feature = "server")]
/// messages delivered to the server
pub enum RecvMessage {
    NewConnection(PeerAddr),
    Request(RequestMessage),
    Submission(SubmissionMessage),
    Question(QuestionMessage),
}
#[cfg(feature = "server")]
/// messages sent from the server
pub enum SendMessage {
    Queue(QueueMessage),
    File(FileMessage),
    EncKey(EncKeyInfo),
}

#[cfg(not(any(feature = "server", feature = "client")))]
pub enum RecvMessage {}
#[cfg(not(any(feature = "server", feature = "client")))]
pub enum SendMessage {}

#[cfg(feature = "server")]
pub enum WBList<T: std::hash::Hash + std::cmp::Eq> {
    Whitelist(HashSet<T>),
    Blacklist(HashSet<T>),
}
#[cfg(feature = "server")]
impl<T: std::hash::Hash + std::cmp::Eq> WBList<T> {
    async fn accept(&self, t: &T) -> bool {
        match self {
            Self::Whitelist(s) => s.contains_async(t).await,
            Self::Blacklist(s) => !s.contains_async(t).await,
        }
    }
    fn new_accept_all() -> Self {
        Self::Blacklist(HashSet::new())
    }
    fn new_reject_all() -> Self {
        Self::Whitelist(HashSet::new())
    }
}
#[cfg(feature = "server")]
pub struct SingleFilter {
    psk_list: WBList<PubSigKey>,
    addr_list: WBList<PeerAddr>,
}
#[cfg(feature = "server")]
impl SingleFilter {
    async fn accept(&self, psk: &PubSigKey, addr: &PeerAddr) -> bool {
        let (accept_psk, accept_addr) =
            join!(self.psk_list.accept(psk), self.addr_list.accept(addr));
        accept_psk && accept_addr
    }
    fn new_psk(psk_white_list: HashSet<PubSigKey>) -> Self {
        Self {
            psk_list: WBList::<PubSigKey>::Whitelist(psk_white_list),
            addr_list: WBList::<PeerAddr>::new_accept_all(),
        }
    }
    fn new_accept_all() -> Self {
        Self {
            psk_list: WBList::<PubSigKey>::new_accept_all(),
            addr_list: WBList::<PeerAddr>::new_accept_all(),
        }
    }
    fn new_reject_all() -> Self {
        Self {
            psk_list: WBList::<PubSigKey>::new_reject_all(),
            addr_list: WBList::<PeerAddr>::new_reject_all(),
        }
    }
}
#[cfg(feature = "server")]
pub struct Filter {
    server: SingleFilter,
    worker: SingleFilter,
    participant: SingleFilter,
    spectator: SingleFilter,
}
#[cfg(feature = "server")]
impl Filter {
    async fn accept(&self, psk: &PubSigKey, addr: &PeerAddr, entity: Entity) -> bool {
        match entity {
            Entity::Server => self.server.accept(psk, addr).await,
            Entity::Worker => self.worker.accept(psk, addr).await,
            Entity::Participant => self.participant.accept(psk, addr).await,
            Entity::Spectator => self.spectator.accept(psk, addr).await,
        }
    }
    pub fn open_server(worker_white_list: HashSet<PubSigKey>) -> Self {
        Self {
            server: SingleFilter::new_reject_all(),
            worker: SingleFilter::new_psk(worker_white_list),
            participant: SingleFilter::new_accept_all(),
            spectator: SingleFilter::new_accept_all(),
        }
    }
}

#[cfg(feature = "client")]
pub struct Filter {}
#[cfg(feature = "client")]
impl Filter {
    async fn accept(&self, _psk: &PubSigKey, _addr: &PeerAddr, _entity: Entity) -> bool {
        false
    }
}
#[cfg(not(any(feature = "client", feature = "server")))]
pub struct Filter {}
#[cfg(not(any(feature = "client", feature = "server")))]
impl Filter {
    async fn accept(&self, _psk: &PubSigKey, _addr: &PeerAddr, _entity: Entity) -> bool {
        false
    }
}

// TODO: disable keepalive if public ip (?)
async fn keepalive(socket: SocketWriter, dest_addr: PeerAddr, mac_key: MacKey) {
    const KEEPALIVE_MSG_SIZE: usize = 13;
    let mut buf = [0u8; KEEPALIVE_MSG_SIZE];
    const KA_DELAY_MIN: Duration = Duration::from_millis(250);
    const KA_DELAY_MAX: Duration = Duration::from_millis(25000);
    loop {
        let message = Message::Net(NetMessage::KeepAlive(
            socket.psk(),
            Macced::new(KeepAliveInner(SystemTime::now()), &mac_key),
        ));
        let interval = if socket.send_to(message, dest_addr, &mut buf).await.is_ok() {
            thread_rng().gen_range(KA_DELAY_MIN..=KA_DELAY_MAX)
        } else {
            KA_DELAY_MIN
        };
        sleep(interval).await;
    }
}
struct Connection {
    ka_ah: Option<AbortHandle>,
    addr: PeerAddr,
    mac_key: MacKey,
    socket: SocketWriter,
}
impl Connection {
    pub async fn start_ka(&mut self) {
        self.abort_ka().await;
        self.ka_ah = Some({
            let socket = self.socket.clone();
            let addr = self.addr;
            let mac_key = self.mac_key;
            tokio::task::spawn(async move { keepalive(socket, addr, mac_key).await }).abort_handle()
        });
    }
    async fn abort_ka(&mut self) {
        if let Some(ah) = self.ka_ah.take() {
            ah.abort();
        }
    }
    pub fn new(addr: PeerAddr, mac_key: MacKey, socket: SocketWriter) -> Self {
        Self {
            ka_ah: None,
            addr,
            mac_key,
            socket,
        }
    }
    pub fn mac_key(&self) -> MacKey {
        self.mac_key
    }
    pub fn addr(&self) -> PeerAddr {
        self.addr
    }
    pub fn set_addr_mackey(&mut self, addr: PeerAddr, mac_key: MacKey) {
        self.addr = addr;
        self.mac_key = mac_key;
    }
}

pub struct Net {
    sw: SocketWriter,
    sr: SocketReader,
    addr_to_psk: HashMap<PeerAddr, PubSigKey>,
    psk_to_addr: HashMap<PubSigKey, PeerAddr>,
    initting: HashMap<(PubSigKey, PeerAddr), (Option<SecKexKey>, AbortHandle)>,
    connections: HashMap<PubSigKey, Connection>,
    keepalivers: HashMap<PubSigKey, u32>,
    inbound_connection_filter: Filter,
}
impl Net {
    pub async fn new(
        ssk: SecSigKey,
        entity: Entity,
        contest_id: ContestId,
        inbound_connection_filter: Filter,
    ) -> Self {
        let (sr, sw) = new_socket("0.0.0.0:0", entity, ssk, contest_id)
            .await
            .unwrap();
        Self {
            sw,
            sr,
            psk_to_addr: HashMap::new(),
            addr_to_psk: HashMap::new(),
            initting: HashMap::new(),
            connections: HashMap::new(),
            keepalivers: HashMap::new(),
            inbound_connection_filter,
        }
    }
    fn psk(&self) -> PubSigKey {
        self.sw.psk()
    }
    async fn handle_net_message(&self, m: NetMessage, peer_addr: PeerAddr) {
        match m {
            NetMessage::Merkle(s) => {
                let peer_id = s.who();
                if let Some((
                    (contest_id, timestamp, peer_pkk, Obfuscated(_peer_addr_local), entity),
                    peer_id,
                )) = s.inner(&peer_id)
                {
                    if is_timestamp_valid(timestamp)
                        && self.sw.contest_id() == contest_id
                        && (self.initting.contains_async(&(peer_id, peer_addr)).await
                            || self
                                .inbound_connection_filter
                                .accept(&peer_id, &peer_addr, entity)
                                .await)
                    {
                        // finalize connection
                        let Some(skk) = self
                            .initting
                            .entry_async((peer_id, peer_addr))
                            .await
                            .or_insert(new_initting(self.sw.clone(), peer_addr).await)
                            .get_mut()
                            .0
                            .take()
                        else {
                            // skk is only taken in this function,
                            // if it's None it means it was already finalized
                            return;
                        };
                        let mac_key = MacKey::from(skk.diffie_hellman(&peer_pkk.into()));

                        let mut occupied = self
                            .connections
                            .entry_async(peer_id)
                            .await
                            .or_insert(Connection::new(peer_addr, mac_key, self.sw.clone()));
                        let c = occupied.get_mut();
                        c.set_addr_mackey(peer_addr, mac_key);
                        c.abort_ka().await;

                        if *self
                            .keepalivers
                            .entry_async(peer_id)
                            .await
                            .or_insert(0)
                            .get()
                            > 0
                        {
                            c.start_ka().await;
                        }
                    }
                }
            }
            NetMessage::KeepAlive(peer_id, macced) => {
                if let Some(mac_key) = self
                    .connections
                    .get_async(&peer_id)
                    .await
                    .map(|x| x.get().mac_key())
                {
                    if let Some(timestamp) = macced.inner(&mac_key) {
                        if is_timestamp_valid(timestamp.0) {
                            if let Some(entry) =
                                self.initting.get_async(&(peer_id, peer_addr)).await
                            {
                                if entry.get().0.is_none() {
                                    let (_k, (_s, ah)) = entry.remove_entry();
                                    ah.abort();
                                } else {
                                    warn!("A connection is re-establishing very quickly(?)");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub async fn update_peer_addr(&self, psk: PubSigKey, addr: PeerAddr) {
        self.psk_to_addr.entry_async(psk).await.insert_entry(addr);
        self.addr_to_psk.entry_async(addr).await.insert_entry(psk);
        if let Some(mut oc) = self.connections.get_async(&psk).await {
            let c = oc.get_mut();
            let mac_key = c.mac_key();
            c.set_addr_mackey(addr, mac_key);
            if *self.keepalivers.entry_async(psk).await.or_insert(0).get() > 0 {
                c.start_ka().await;
            }
        }
    }
    pub async fn inc_keepalive(&self, psk: PubSigKey) {
        let cnt = {
            let entry = self.keepalivers.entry_async(psk).await;
            let mut occupied = entry.or_insert(0);
            let ka = occupied.get_mut();
            *ka += 1;
            *ka
        };
        if cnt == 1 {
            if let Some(mut c) = self.connections.get_async(&psk).await {
                c.get_mut().start_ka().await;
            } else if let Some(addr_entry) = self.psk_to_addr.get_async(&psk).await {
                let addr = *addr_entry.get();
                let _ = self
                    .initting
                    .insert_async((psk, addr), new_initting(self.sw.clone(), addr).await)
                    .await;
            }
        }
    }
    pub async fn dec_keepalive(&self, psk: PubSigKey) {
        let cnt = {
            let entry = self.keepalivers.entry_async(psk).await;
            let mut occupied = entry.or_insert(0);
            let ka = occupied.get_mut();
            if *ka != 0 {
                *ka -= 1;
            } else {
                error!("decreasing keepalive counter when it was already 0");
            }
            *ka
        };
        if cnt == 0 {
            if let Some(mut c) = self.connections.get_async(&psk).await {
                c.get_mut().abort_ka().await;
            }
        }
    }
}
// server only
#[cfg(feature = "server")]
impl Net {
    pub async fn recv(&self, buf: &mut [u8]) -> (RecvMessage, PubSigKey) {
        loop {
            let (m, addr) = self.sr.recv_from(buf).await;
            match m {
                Message::Net(nm) => {
                    self.handle_net_message(nm, addr).await;
                }
                Message::Request(rm) => {
                    if let Some(psk) = self.addr_to_psk.get_async(&addr).await.map(|x| *x.get()) {
                        if let Some(mac_key) = self
                            .connections
                            .get_async(&psk)
                            .await
                            .map(|x| x.get().mac_key())
                        {
                            if let Some(inner) = rm.inner(&mac_key) {
                                return (RecvMessage::Request(inner), psk);
                            }
                        }
                    }
                }
                Message::Submission(sm) => {
                    if let Some(psk) = self.addr_to_psk.get_async(&addr).await.map(|x| *x.get()) {
                        if let Some(mac_key) = self
                            .connections
                            .get_async(&psk)
                            .await
                            .map(|x| x.get().mac_key())
                        {
                            if let Some(inner) = sm.inner(&mac_key) {
                                return (RecvMessage::Submission(inner), psk);
                            }
                        }
                    }
                }
                Message::Question(qm) => {
                    if let Some(psk) = self.addr_to_psk.get_async(&addr).await.map(|x| *x.get()) {
                        if let Some(mac_key) = self
                            .connections
                            .get_async(&psk)
                            .await
                            .map(|x| x.get().mac_key())
                        {
                            if let Some(inner) = qm.inner(&mac_key) {
                                return (RecvMessage::Question(inner), psk);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    pub async fn send(&self, m: SendMessage, psk: PubSigKey, buf: &mut [u8]) -> anyhow::Result<()> {
        let mac_key = self
            .connections
            .get_async(&psk)
            .await
            .ok_or(anyhow::anyhow!(
                "Trying to send message, but there is no connection"
            ))?
            .get()
            .mac_key();
        let addr = *self
            .psk_to_addr
            .get_async(&psk)
            .await
            .ok_or(anyhow::anyhow!(
                "Trying to send message, could not find addr from psk"
            ))?
            .get();
        let message = match m {
            SendMessage::Queue(m) => {
                Message::Queue(Macced::new(Signed::new((m, ()), &self.sw.ssk()), &mac_key))
            }
            SendMessage::File(m) => Message::File(Macced::new(m, &mac_key)),
            SendMessage::EncKey(m) => Message::EncKey(Macced::new(m, &mac_key)),
        };
        self.sw.send_to(message, addr, buf).await
    }
}
// client only
#[cfg(feature = "client")]
impl Net {
    pub async fn recv(&self, server_psk: PubSigKey, buf: &mut [u8]) -> (RecvMessage, PubSigKey) {
        loop {
            let (m, addr) = self.sr.recv_from(buf).await;
            match m {
                Message::Net(nm) => {
                    self.handle_net_message(nm, addr).await;
                }
                Message::Queue(qm) => {
                    if let Some(psk) = self.addr_to_psk.get_async(&addr).await.map(|x| *x.get()) {
                        if let Some(mac_key) = self
                            .connections
                            .get_async(&psk)
                            .await
                            .map(|x| x.get().mac_key())
                        {
                            if let Some(signed) = qm.inner(&mac_key) {
                                if let Some(inner) = signed.inner(&server_psk) {
                                    return (RecvMessage::Queue(inner.0), psk);
                                }
                            }
                        }
                    }
                }
                Message::File(fm) => {
                    if let Some(psk) = self.addr_to_psk.get_async(&addr).await.map(|x| *x.get()) {
                        if let Some(mac_key) = self
                            .connections
                            .get_async(&psk)
                            .await
                            .map(|x| x.get().mac_key())
                        {
                            if let Some(inner) = fm.inner(&mac_key) {
                                return (RecvMessage::File(inner), psk);
                            }
                        }
                    }
                }
                Message::Request(rm) => {
                    if let Some(psk) = self.addr_to_psk.get_async(&addr).await.map(|x| *x.get()) {
                        if let Some(mac_key) = self
                            .connections
                            .get_async(&psk)
                            .await
                            .map(|x| x.get().mac_key())
                        {
                            if let Some(inner) = rm.inner(&mac_key) {
                                return (RecvMessage::Request(inner), psk);
                            }
                        }
                    }
                }
                Message::EncKey(em) => {
                    if let Some(psk) = self.addr_to_psk.get_async(&addr).await.map(|x| *x.get()) {
                        if let Some(mac_key) = self
                            .connections
                            .get_async(&psk)
                            .await
                            .map(|x| x.get().mac_key())
                        {
                            if let Some(inner) = em.inner(&mac_key) {
                                return (RecvMessage::EncKey(inner), psk);
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
    pub async fn send(&self, m: SendMessage, psk: PubSigKey, buf: &mut [u8]) -> anyhow::Result<()> {
        let mac_key = self
            .connections
            .get_async(&psk)
            .await
            .ok_or(anyhow::anyhow!(
                "Trying to send message, but there is no connection"
            ))?
            .get()
            .mac_key();
        let addr = *self
            .psk_to_addr
            .get_async(&psk)
            .await
            .ok_or(anyhow::anyhow!(
                "Trying to send message, could not find addr from psk"
            ))?
            .get();
        let message = match m {
            SendMessage::File(m) => Message::File(Macced::new(m, &mac_key)),
            SendMessage::Request(m) => Message::Request(Macced::new(m, &mac_key)),
            SendMessage::Submission(m) => Message::Submission(Macced::new(m, &mac_key)),
            SendMessage::Question(m) => Message::Question(Macced::new(m, &mac_key)),
        };
        self.sw.send_to(message, addr, buf).await
    }
}

async fn new_initting(
    socket: SocketWriter,
    peer_addr: PeerAddr,
) -> (Option<SecKexKey>, AbortHandle) {
    let skk = SecKexKey::random_from_rng(thread_rng());
    let abort_handle = task::spawn(send_kex_loop(socket, (&skk).into(), peer_addr)).abort_handle();
    (Some(skk), abort_handle)
}

async fn send_kex_loop(socket: SocketWriter, pkk: PubKexKey, peer_addr: PeerAddr) {
    let mut buf = [0u8; 153];
    let contest_id = socket.contest_id();
    let obf_addr = Obfuscated(socket.own_addr().unwrap());
    let ssk = socket.ssk();
    let psk = socket.psk();
    loop {
        let _ = socket
            .send_to(
                Message::Net(NetMessage::Merkle(Signed::new(
                    (
                        (
                            contest_id,
                            SystemTime::now(),
                            pkk,
                            obf_addr,
                            socket.entity(),
                        ),
                        psk,
                    ),
                    &ssk,
                ))),
                peer_addr,
                &mut buf,
            )
            .await;
        let interval =
            thread_rng().gen_range(Duration::from_millis(40)..Duration::from_millis(400));
        sleep(interval).await;
    }
}
