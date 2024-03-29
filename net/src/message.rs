// Here I define the message type for networking
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha8;
use core::hash::Hash;
use derive_more::{From, Into};
use ed25519_dalek::Signer;
use ordered_float::NotNan;
use speedy::{Context, LittleEndian, Readable, Reader, Writable, Writer};
use std::marker::PhantomData;
use std::net::{IpAddr, SocketAddr, SocketAddrV4, SocketAddrV6};
use std::str::FromStr;
use std::time::{Duration, SystemTime};

pub type Timestamp = SystemTime;
pub fn is_timestamp_valid(timestamp: Timestamp) -> bool {
    let now = SystemTime::now();
    if timestamp > now {
        timestamp.duration_since(now).unwrap() < Duration::from_secs(20)
    } else {
        now.duration_since(timestamp).unwrap() < Duration::from_secs(40)
    }
}

pub type ContestId = u128;
pub type ProblemId = u32;
pub type SecSigKey = ed25519_dalek::SigningKey;
pub type SecKexKey = x25519_dalek::EphemeralSecret;

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Readable, Writable)]
#[repr(u8)]
#[speedy(tag_type = u8)]
pub enum Entity {
    Server,
    Worker,
    Participant,
    Spectator,
}
impl FromStr for Entity {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "server" => Ok(Self::Server),
            "worker" => Ok(Self::Worker),
            "participant" => Ok(Self::Participant),
            "spectator" => Ok(Self::Spectator),
            _ => Err(anyhow::anyhow!(
                "Entity must be one of: server, worker, participant, spectator"
            )),
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into)]
pub struct EncKey(chacha20::Key);
impl<'a, C> Readable<'a, C> for EncKey
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut octets = [0; 32];
        reader.read_bytes(&mut octets)?;
        if !reader.endianness().conversion_necessary() {
            octets.reverse();
        }
        Ok(EncKey(octets.into()))
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        12
    }
}
impl<C> Writable<C> for EncKey
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut octets: [u8; 32] = self.0.into();
        if !writer.endianness().conversion_necessary() {
            octets.reverse();
        }
        writer.write_bytes(&octets)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(12)
    }
}
impl EncKey {
    #[cfg(test)]
    fn dummy() -> Self {
        Self([42; 32].into())
    }
    pub fn random() -> Self {
        Self(rand::random::<[u8; 32]>().into())
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into)]
pub struct EncNonce(chacha20::Nonce);
impl<'a, C> Readable<'a, C> for EncNonce
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut octets = [0; 12];
        reader.read_bytes(&mut octets)?;
        if !reader.endianness().conversion_necessary() {
            octets.reverse();
        }
        Ok(EncNonce(octets.into()))
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        12
    }
}
impl<C> Writable<C> for EncNonce
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut octets: [u8; 12] = self.0.into();
        if !writer.endianness().conversion_necessary() {
            octets.reverse();
        }
        writer.write_bytes(&octets)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(12)
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, From, Into)]
pub struct PubKexKey(pub x25519_dalek::PublicKey);
impl<'a, C> Readable<'a, C> for PubKexKey
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut octets = [0; 32];
        reader.read_bytes(&mut octets)?;
        if !reader.endianness().conversion_necessary() {
            octets.reverse();
        }
        Ok(PubKexKey(x25519_dalek::PublicKey::from(octets)))
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        32
    }
}
impl<C> Writable<C> for PubKexKey
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut octets = self.0.to_bytes();
        if !writer.endianness().conversion_necessary() {
            octets.reverse();
        }
        writer.write_bytes(&octets)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(32)
    }
}
impl From<&SecKexKey> for PubKexKey {
    fn from(skk: &SecKexKey) -> Self {
        Self(skk.into())
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, From, Into)]
pub struct PubSigKey(ed25519_dalek::VerifyingKey);
impl PubSigKey {
    #[cfg(test)]
    pub fn dummy() -> Self {
        Self(ed25519_dalek::VerifyingKey::from_bytes(&[42u8; 32]).unwrap())
    }
}
impl From<&SecSigKey> for PubSigKey {
    fn from(ssk: &SecSigKey) -> Self {
        Self(ssk.verifying_key())
    }
}
impl FromStr for PubSigKey {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let b = hex::decode(s)?;
        Ok(Self(ed25519_dalek::VerifyingKey::from_bytes(
            &b.try_into()
                .map_err(|_| anyhow::anyhow!("error converting slice to [u8;32]"))?,
        )?))
    }
}
impl<'a, C> Readable<'a, C> for PubSigKey
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut octets = [0; 32];
        reader.read_bytes(&mut octets)?;
        if !reader.endianness().conversion_necessary() {
            octets.reverse();
        }
        match ed25519_dalek::VerifyingKey::from_bytes(&octets) {
            Ok(x) => Ok(PubSigKey(x)),
            Err(_) => Err(speedy::Error::custom("Could not parse public ed25519 key").into()),
        }
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        32
    }
}
impl<C> Writable<C> for PubSigKey
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut octets = self.0.to_bytes();
        if !writer.endianness().conversion_necessary() {
            octets.reverse();
        }
        writer.write_bytes(&octets)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(32)
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into)]
pub struct MacKey([u8; 32]); //(x25519_dalek::SharedSecret);
impl MacKey {
    #[cfg(test)]
    pub fn dummy() -> Self {
        use x25519_dalek::{EphemeralSecret, PublicKey};

        let alice_secret = EphemeralSecret::random();
        let alice_public = PublicKey::from(&alice_secret);

        let bob_secret = EphemeralSecret::random();
        let bob_public = PublicKey::from(&bob_secret);

        let alice_shared_secret = alice_secret.diffie_hellman(&bob_public);
        let bob_shared_secret = bob_secret.diffie_hellman(&alice_public);
        assert_eq!(alice_shared_secret.as_bytes(), bob_shared_secret.as_bytes());

        Self(alice_shared_secret.to_bytes())
    }
}
impl From<x25519_dalek::SharedSecret> for MacKey {
    fn from(ss: x25519_dalek::SharedSecret) -> Self {
        Self(ss.to_bytes())
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into)]
pub struct Signature(ed25519_dalek::Signature);
impl<'a, C> Readable<'a, C> for Signature
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut octets = [0; 64];
        reader.read_bytes(&mut octets)?;
        if !reader.endianness().conversion_necessary() {
            octets.reverse();
        }
        Ok(Signature(ed25519_dalek::Signature::from_bytes(&octets)))
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        64
    }
}
impl<C> Writable<C> for Signature
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut octets = self.0.to_bytes();
        if !writer.endianness().conversion_necessary() {
            octets.reverse();
        }
        writer.write_bytes(&octets)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(64)
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, From, Into)]
pub struct Mac(pub blake3::Hash);
impl<'a, C> Readable<'a, C> for Mac
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut octets = [0; 32];
        reader.read_bytes(&mut octets)?;
        if !reader.endianness().conversion_necessary() {
            octets.reverse();
        }
        Ok(Mac(blake3::Hash::from_bytes(octets)))
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        32
    }
}
impl<C> Writable<C> for Mac
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut octets = *self.0.as_bytes();
        if !writer.endianness().conversion_necessary() {
            octets.reverse();
        }
        writer.write_bytes(&octets)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(32)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable, Copy)]
pub struct Signed<T, W>
where
    T: Writable<LittleEndian>,
    W: Writable<LittleEndian> + Copy,
{
    data: (T, W),
    signature: Signature,
}
impl<T, W> Signed<T, W>
where
    T: Writable<LittleEndian>,
    W: Writable<LittleEndian> + Copy,
{
    pub fn check(&self, pk: &PubSigKey) -> bool {
        if let Ok(buf) = self.data.write_to_vec() {
            pk.0.verify_strict(&buf, &self.signature.0).is_ok()
        } else {
            false
        }
    }
    pub fn inner(self, pk: &PubSigKey) -> Option<(T, W)> {
        if self.check(pk) {
            Some(self.data)
        } else {
            None
        }
    }
    pub fn who(&self) -> W {
        self.data.1
    }
    pub fn new(data: (T, W), sk: &SecSigKey) -> Self {
        let buf = data.write_to_vec().unwrap();
        let signature = sk.sign(&buf);
        Self {
            data,
            signature: Signature(signature),
        }
    }
}
#[derive(PartialEq, Eq, Debug, Copy, Clone, Readable, Writable)]
pub struct Macced<T: Writable<LittleEndian>> {
    data: T,
    mac: Mac,
}
impl<T> Macced<T>
where
    T: Writable<LittleEndian>,
{
    pub fn check(&self, key: &MacKey) -> bool {
        if let Ok(buf) = self.data.write_to_vec() {
            self.mac.0 == blake3::keyed_hash(&key.0, &buf)
        } else {
            false
        }
    }
    pub fn inner(self, key: &MacKey) -> Option<T> {
        if self.check(key) {
            Some(self.data)
        } else {
            None
        }
    }
    pub fn new(data: T, key: &MacKey) -> Self {
        let buf = data.write_to_vec().unwrap();
        let h = blake3::keyed_hash(&key.0, &buf);
        Self { data, mac: Mac(h) }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, From)]
pub struct Obfuscated<T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian>>(pub T);
const OBFUSCATION_BYTES: [u8; 32] = [
    185, 174, 209, 69, 42, 248, 31, 131, 3, 22, 177, 242, 148, 120, 109, 165, 163, 207, 114, 158,
    146, 106, 82, 236, 83, 188, 149, 239, 189, 232, 255, 90,
];
impl<T> Obfuscated<T> where T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian> {
    pub fn inner(self) -> T {
        self.0
    }
}
impl<'a, C, T> Readable<'a, C> for Obfuscated<T>
where
    C: Context,
    T: Readable<'a, C>,
    T: Writable<LittleEndian> + for<'b> Readable<'b, LittleEndian>,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut data: Vec<u8> = reader.read_value()?;
        for (i, v) in data.iter_mut().enumerate() {
            *v ^= OBFUSCATION_BYTES[i & 31];
        }
        Ok(Obfuscated(T::read_from_buffer(&data)?))
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        4 + <T as Readable<'a, C>>::minimum_bytes_needed()
    }
}
impl<C, T> Writable<C> for Obfuscated<T>
where
    C: Context,
    T: Writable<LittleEndian> + for<'b> Readable<'b, LittleEndian>,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut data = self.0.write_to_vec()?;
        for (i, v) in data.iter_mut().enumerate() {
            *v ^= OBFUSCATION_BYTES[i & 31];
        }
        writer.write_value(&data)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(4 + self.0.bytes_needed()?)
    }
}

#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Encrypted<T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian>> {
    data: Vec<u8>,
    nonce: EncNonce,
    _phantom: PhantomData<T>,
}
impl<'a, C, T> Readable<'a, C> for Encrypted<T>
where
    C: Context,
    T: Readable<'a, C>,
    T: Writable<LittleEndian> + for<'b> Readable<'b, LittleEndian>,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let data: Vec<u8> = reader.read_value()?;
        let nonce: EncNonce = reader.read_value()?;
        Ok(Encrypted {
            data,
            nonce,
            _phantom: PhantomData,
        })
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        <Vec<u8> as Readable<'a, C>>::minimum_bytes_needed()
            + <EncNonce as Readable<'a, C>>::minimum_bytes_needed()
    }
}
impl<C, T> Writable<C> for Encrypted<T>
where
    C: Context,
    T: Writable<LittleEndian> + for<'b> Readable<'b, LittleEndian>,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        writer.write_value(&self.data)?;
        writer.write_value(&self.nonce)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(<Vec<u8> as Writable<C>>::bytes_needed(&self.data)?
            + <EncNonce as Writable<C>>::bytes_needed(&self.nonce)?)
    }
}
impl<T> Encrypted<T>
where
    T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian>,
{
    pub fn inner(self, key: &EncKey) -> Option<T> {
        let mut cipher = ChaCha8::new(&key.0, &self.nonce.into());
        let mut buf = self.data;
        cipher.apply_keystream(&mut buf);
        T::read_from_buffer(&buf).ok()
    }
    pub fn new(data: T, key: &EncKey) -> Self {
        let nonce: EncNonce = EncNonce(rand::random::<[u8; 12]>().into()); //TODO: is this good?
        let mut cipher = ChaCha8::new(&key.0, &nonce.into());
        let mut buf = data.write_to_vec().unwrap();
        cipher.apply_keystream(&mut buf);
        Encrypted {
            data: buf,
            nonce,
            _phantom: PhantomData,
        }
    }
}

// speedy Readable and Writable derives are currently bugged with const generics
#[derive(PartialEq, Eq, Debug, Clone, Copy)] //, Readable, Writable)]
pub struct SizedEncrypted<T, const N: usize>
where
    T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian>,
{
    data: [u8; N],
    nonce: EncNonce,
    _phantom: PhantomData<T>,
}
impl<'a, C, T, const N: usize> Readable<'a, C> for SizedEncrypted<T, N>
where
    C: Context,
    T: Readable<'a, C>,
    T: Writable<LittleEndian> + for<'b> Readable<'b, LittleEndian>,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut data = [0u8; N];
        reader.read_bytes(&mut data)?;
        let nonce: EncNonce = reader.read_value()?;
        Ok(SizedEncrypted {
            data,
            nonce,
            _phantom: PhantomData,
        })
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        N + <EncNonce as Readable<'a, C>>::minimum_bytes_needed()
    }
}
impl<C, T, const N: usize> Writable<C> for SizedEncrypted<T, N>
where
    C: Context,
    T: Writable<LittleEndian> + for<'b> Readable<'b, LittleEndian>,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        writer.write_bytes(&self.data)?;
        writer.write_value(&self.nonce)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(N + <EncNonce as Writable<C>>::bytes_needed(&self.nonce)?)
    }
}
impl<T, const N: usize> SizedEncrypted<T, N>
where
    T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian>,
{
    pub fn inner(self, key: &EncKey) -> Option<T> {
        let mut cipher = ChaCha8::new(&key.0, &self.nonce.into());
        let mut buf = self.data;
        cipher.apply_keystream(&mut buf);
        T::read_from_buffer(&buf).ok()
    }
    pub fn new(data: T, key: &EncKey) -> Self {
        let nonce: EncNonce = EncNonce(rand::random::<[u8; 12]>().into()); //TODO: is this good?
        let mut cipher = ChaCha8::new(&key.0, &nonce.into());
        let mut buf = [0u8; N];
        data.write_to_buffer(&mut buf).unwrap();
        cipher.apply_keystream(&mut buf);
        SizedEncrypted {
            data: buf,
            nonce,
            _phantom: PhantomData,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into)]
pub struct FileChunk(pub [u8; FILE_CHUNK_SIZE]);
impl<'a, C> Readable<'a, C> for FileChunk
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let mut octets = [0u8; FILE_CHUNK_SIZE];
        reader.read_bytes(&mut octets)?;
        if !reader.endianness().conversion_necessary() {
            octets.reverse();
        }
        Ok(FileChunk(octets))
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        FILE_CHUNK_SIZE
    }
}
impl<C> Writable<C> for FileChunk
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        let mut octets: [u8; FILE_CHUNK_SIZE] = self.0;
        if !writer.endianness().conversion_necessary() {
            octets.reverse();
        }
        writer.write_bytes(&octets)
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(FILE_CHUNK_SIZE)
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, PartialOrd, Ord)]
pub struct SubScore(NotNan<f64>);
impl TryFrom<f64> for SubScore {
    type Error = ();
    fn try_from(f: f64) -> Result<Self, Self::Error> {
        Ok(SubScore(NotNan::new(f).map_err(|_| ())?))
    }
}
impl From<SubScore> for f64 {
    fn from(f: SubScore) -> Self {
        f.0.into_inner()
    }
}
impl<'a, C> Readable<'a, C> for SubScore
where
    C: Context,
{
    #[inline]
    fn read_from<R: Reader<'a, C>>(reader: &mut R) -> Result<Self, C::Error> {
        let v: f64 = reader.read_value()?;
        if (0f64..=1f64).contains(&v) {
            Ok(Self::try_from(v).unwrap())
        } else {
            Err(speedy::Error::custom("score not in 0..=1").into())
        }
    }
    #[inline]
    fn minimum_bytes_needed() -> usize {
        8
    }
}
impl<C> Writable<C> for SubScore
where
    C: Context,
{
    #[inline]
    fn write_to<W>(&self, writer: &mut W) -> Result<(), C::Error>
    where
        W: ?Sized + Writer<C>,
    {
        writer.write_value(&f64::from(*self))
    }
    #[inline]
    fn bytes_needed(&self) -> Result<usize, C::Error> {
        Ok(8)
    }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Copy, Clone, Hash, Readable, Writable)]
pub struct PeerAddr {
    ip: IpAddr,
    port: u16,
}
impl PeerAddr {
    pub fn new(ip: IpAddr, port: u16) -> Self {
        Self { ip, port }
    }
}
impl FromStr for PeerAddr {
    type Err = std::net::AddrParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(std::net::SocketAddr::from_str(s)?.into())
    }
}
impl From<std::net::SocketAddr> for PeerAddr {
    fn from(addr: std::net::SocketAddr) -> Self {
        Self {
            ip: addr.ip(),
            port: addr.port(),
        }
    }
}
impl From<PeerAddr> for std::net::SocketAddr {
    fn from(addr: PeerAddr) -> std::net::SocketAddr {
        match addr.ip {
            IpAddr::V4(ip) => SocketAddr::V4(SocketAddrV4::new(ip, addr.port)),
            IpAddr::V6(ip) => SocketAddr::V6(SocketAddrV6::new(ip, addr.port, 0, 0)),
        }
    }
}

// to avoid ip fragmentation
pub const MAX_PACKET_SIZE: usize = 1280;
pub const MAX_MESSAGE_SIZE: usize = MAX_PACKET_SIZE - 48; // 40 ipv6 header, 8 udp header
                                                          // check at compile time that a message (in rust memory, not the actual message being transmitted)
                                                          // fits in the maximum size
                                                          //const _: () = [(); 1][(core::mem::size_of::<Message>() <= MAX_MESSAGE_SIZE) as usize ^ 1];

#[allow(clippy::large_enum_variant)]
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
#[repr(u8)]
#[speedy(tag_type = u8)]
pub enum Message {
    Net(NetMessage),
    Queue(Macced<Signed<QueueMessage, ()>>),
    File(Macced<FileMessage>),
    EncKey(Macced<EncKeyInfo>),
    Request(Macced<RequestMessage>),
    Submission(Macced<SubmissionMessage>),
    Question(Macced<QuestionMessage>),
}

// Net
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable, Copy)]
#[repr(u8)]
#[speedy(tag_type = u8)]
pub enum NetMessage {
    // Entity here is only really useful when connecting to server
    // for choosing to be participant, spectator or whatever
    Merkle(
        Signed<
            (
                ContestId,
                Timestamp,
                PubKexKey,
                Obfuscated<PeerAddr>,
                Entity,
            ),
            PubSigKey,
        >,
    ),
    KeepAlive(PubSigKey, Macced<KeepAliveInner>),
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable, Copy)]
pub struct KeepAliveInner(pub Timestamp);

pub type QueueMessageId = u32;
// Queue
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QueueMessage {
    pub id: QueueMessageId,
    pub timestamp: Timestamp,
    pub message: QueueMessageInner,
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
#[repr(u8)]
#[speedy(tag_type = u8)]
pub enum QueueMessageInner {
    Submission(QSubmission),
    EvaluationRequest(QEvaluationRequest),
    Evaluation(QEvaluation),
    EvaluationProof(QEvaluationProof),
    ProblemDesc(QProblemDesc),
    Announcement(QAnnouncement),
    PublicKey(EncKeyInfo),
    PeerInfo(QPeerInfo),
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QPeerInfo {
    pub psk: PubSigKey,
    pub addr: Obfuscated<PeerAddr>,
    pub entity: Entity,
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QAnnouncement {
    #[speedy(length_type=u8)]
    pub text: String,
    pub context: Option<ProblemId>,
}
#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Readable, Writable)]
pub struct SubmissionId {
    pub submitter: PubSigKey,
    pub problem_id: ProblemId,
    pub file_id: FileHash,
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QEvaluationRequest {
    pub submission_id: SubmissionId,
    pub evaluators: Vec<PubSigKey>,
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QSubmission {
    pub submitter: PubSigKey,
    pub problem_id: ProblemId,
    pub file_desc: QFileDesc,
}
impl QSubmission {
    pub fn submission_id(&self) -> SubmissionId {
        SubmissionId {
            submitter: self.submitter,
            problem_id: self.problem_id,
            file_id: self.file_desc.hash,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Copy, Hash, Readable, Writable)]
pub struct EvaluationId {
    pub submission_id: SubmissionId,
    pub evaluator: PubSigKey,
}
impl EvaluationId {
    pub fn get_public_hash_data(&self) -> [u8; 32] {
        let v = self.write_to_vec().unwrap();
        blake3::hash(&v).into()
    }
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QEvaluation {
    pub evaluation_id: EvaluationId,
    pub score: SubScore,
    pub detailhs_hash: DetailHash,
}
impl QEvaluation {
    pub fn new(evp: QEvaluationProof, score: SubScore) -> Self {
        let data = evp.evaluation_id.get_public_hash_data();
        let key = evp.detailhs.0;
        let detailhs_hash = Mac(blake3::keyed_hash(&key.into(), &data));
        Self {
            evaluation_id: evp.evaluation_id,
            score,
            detailhs_hash,
        }
    }
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QEvaluationProof {
    pub evaluation_id: EvaluationId,
    pub detailhs: DetailHash,
}
impl QEvaluationProof {
    pub fn check(&self, ev: &QEvaluation) -> bool {
        self.evaluation_id != ev.evaluation_id && {
            let data = ev.evaluation_id.get_public_hash_data();
            let key = self.detailhs.0;
            let p = blake3::keyed_hash(&key.into(), &data);
            p == ev.detailhs_hash.0
        }
    }
    pub fn hash(&self) -> DetailHash {
        let data = self.evaluation_id.get_public_hash_data();
        let key = self.detailhs.0;
        Mac(blake3::keyed_hash(&key.into(), &data))
    }
}
pub type DetailHash = Mac;

#[derive(PartialEq, Eq, Debug, Clone, Hash, Readable, Writable)]
#[repr(u8)]
#[speedy(tag_type = u8)]
pub enum EncKeyId {
    // you should have the enc key if:
    CustomPublic(u32), // the contest master decided to publish this key to the queue (note: can use this for contest start/end)
    IsEntity(Entity),  // you are of that entity type
    IsClient(PubSigKey), // you are that specific client
    ProblemSolved(ProblemId), // you solved that problem
    Or(Vec<EncKeyId>), // you have any of these requirements
    And(Vec<EncKeyId>), // you have all of these requirements
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct EncKeyInfo {
    pub id: EncKeyId,
    pub key: EncKey,
}
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QProblemDesc {
    pub id: ProblemId,
    pub statement: QFileDesc,
    pub generator_file: QFileDesc,
    pub scorer_file: QFileDesc, // TODO: give unique names to all the scoring phases(?)
    pub n_testcases: u32,       // TODO: do we care about encrypting this?
}

pub type FileHash = Mac;
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QFileDesc {
    pub hash: FileHash,
    pub size: u32,                                      // length in bytes
    pub key_encrypting_key: EncKeyId, // id of the key used to encrypt the encrypting key
    pub enc_encrypting_key: SizedEncrypted<EncKey, 32>, // encrypted key used to encrypt the file
}

// - message tag - mac - hash - offset - nonce
pub const FILE_CHUNK_SIZE: usize = MAX_MESSAGE_SIZE - 1 - 32 - 32 - 4 - 12;
// File
#[derive(PartialEq, Eq, Debug, Clone, Copy, Readable, Writable)]
pub struct FileMessage {
    pub hash: FileHash,
    pub piece: u32, //inc id (offset = piece*FILE_CHUNK_SIZE)
    pub data: SizedEncrypted<FileChunk, FILE_CHUNK_SIZE>,
}

// Question
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QuestionMessage {
    #[speedy(length_type=u8)]
    pub text: String,
    pub context: Option<ProblemId>,
}

// Submission
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct SubmissionMessage {
    pub problem_id: ProblemId,
    pub file_id: FileHash,
    pub file_size: u32,
    enc_key: EncKey,
}

// Request
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
#[repr(u8)]
#[speedy(tag_type = u8)]
pub enum RequestMessage {
    File(Vec<(u32, u32)>),  //[id,id]
    Queue(Vec<(u32, u32)>), //[id,id]
    EncKey(EncKeyId),
}

#[cfg(test)]
mod test {
    use super::*;
    fn get_dummy_mac() -> Mac {
        Mac([42; 32].into())
    }
    #[test]
    fn file_message() {
        let file = FileChunk([42u8; FILE_CHUNK_SIZE]);
        let enc_key = EncKey::dummy();
        let mac_key = MacKey::dummy();

        let hash = get_dummy_mac();
        let offset = 0u32;
        let data = SizedEncrypted::<_, FILE_CHUNK_SIZE>::new(file, &enc_key);

        let file_message = FileMessage { hash, offset, data };
        let macced = Macced::new_from_mac_key_test(file_message, &mac_key);
        let message = Message::File(macced);

        let ser = message.write_to_vec().unwrap();
        assert_eq!(ser.len(), MAX_MESSAGE_SIZE);

        let unser = Message::read_from_buffer(&ser).unwrap();
        assert_eq!(message, unser);

        let unmacced = macced.inner_from_mac_key_test(&mac_key).unwrap();

        assert_eq!(file_message, unmacced);

        let unenced = file_message.data.inner(&enc_key).unwrap();

        assert_eq!(file, unenced);
    }
    #[test]
    fn obfuscated_ipv6() {
        let socket: Obfuscated<PeerAddr> = Obfuscated(PeerAddr::from(
            "[::1]:8080".parse::<std::net::SocketAddr>().unwrap(),
        ));
        let ser = socket.write_to_vec().unwrap();
        let unser = Obfuscated::<PeerAddr>::read_from_buffer(&ser).unwrap();
        assert_eq!(socket, unser);
    }
    #[test]
    fn obfuscated_ipv4() {
        let socket: Obfuscated<PeerAddr> = Obfuscated(PeerAddr::from(
            "127.0.0.1:8080".parse::<std::net::SocketAddr>().unwrap(),
        ));
        let ser = socket.write_to_vec().unwrap();
        let unser = Obfuscated::<PeerAddr>::read_from_buffer(&ser).unwrap();
        assert_eq!(socket, unser);
    }
}
