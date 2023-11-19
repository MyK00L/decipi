// Here I define the message type for networking
use chacha20::cipher::{KeyIvInit, StreamCipher};
use chacha20::ChaCha8;
use derive_more::{Deref, DerefMut, From, Into};
use ed25519_dalek::Signer;
use speedy::{BigEndian, Context, Endianness, LittleEndian, Readable, Reader, Writable, Writer};
use std::net::SocketAddr;
use std::time::SystemTime;
use tokio::time::Duration;

pub type Timestamp = SystemTime;
pub type ContestId = u128;
pub type PeerId = u32;
pub type SecSigKey = ed25519_dalek::SigningKey;

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into, Deref, DerefMut)]
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

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into, Deref, DerefMut)]
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

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into, Deref, DerefMut)]
pub struct PubKexKey(x25519_dalek::PublicKey);
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

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into, Deref, DerefMut)]
pub struct PubSigKey(ed25519_dalek::VerifyingKey);
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

#[derive(From, Into, Deref, DerefMut)]
pub struct MacKey(x25519_dalek::SharedSecret);
#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into, Deref, DerefMut)]
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

#[derive(PartialEq, Eq, Debug, Copy, Clone, From, Into, Deref, DerefMut)]
pub struct Mac(blake3::Hash);
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
            pk.verify_strict(&buf, &self.signature).is_ok()
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
            self.mac.0 == blake3::keyed_hash(key.as_bytes(), &buf)
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
        let h = blake3::keyed_hash(key.as_bytes(), &buf);
        Self { data, mac: Mac(h) }
    }
}
#[derive(PartialEq, Eq, Debug, Clone)]
pub struct Encrypted<T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian>> {
    data: Vec<u8>,
    nonce: EncNonce,
    _phantom: std::marker::PhantomData<T>,
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
            _phantom: std::marker::PhantomData,
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
        let mut cipher = ChaCha8::new(key, &self.nonce);
        let mut buf = self.data;
        cipher.apply_keystream(&mut buf);
        T::read_from_buffer(&buf).ok()
    }
    pub fn new(data: T, key: &EncKey) -> Self {
        let nonce: EncNonce = EncNonce([0x24; 12].into()); //TODO
        let mut cipher = ChaCha8::new(key, &nonce);
        let mut buf = data.write_to_vec().unwrap();
        cipher.apply_keystream(&mut buf);
        Encrypted {
            data: buf,
            nonce,
            _phantom: std::marker::PhantomData,
        }
    }
}

// speedy Readable and Writable derives are currently bugged with const generics
#[derive(PartialEq, Eq, Debug, Clone)] //, Readable, Writable)]
pub struct SizedEncrypted<T, const N: usize>
where
    T: Writable<LittleEndian> + for<'a> Readable<'a, LittleEndian>,
{
    data: [u8; N],
    nonce: EncNonce,
    _phantom: std::marker::PhantomData<T>,
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
            _phantom: std::marker::PhantomData,
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
        let mut cipher = ChaCha8::new(&key.0, &self.nonce);
        let mut buf = self.data;
        cipher.apply_keystream(&mut buf);
        T::read_from_buffer(&buf).ok()
    }
    pub fn new(data: T, key: &EncKey) -> Self {
        let nonce: EncNonce = EncNonce([0x24; 12].into()); //TODO
        let mut cipher = ChaCha8::new(&key.0, &nonce);
        let mut buf = [0u8; N];
        data.write_to_buffer(&mut buf).unwrap();
        cipher.apply_keystream(&mut buf);
        SizedEncrypted {
            data: buf,
            nonce,
            _phantom: std::marker::PhantomData,
        }
    }
}

#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub enum Message {
    KeepAlive(KeepAliveMessage),
    Init(InitMessage),
    Queue(Macced<QueueMessage>),
    File(Macced<FileMessage>),
    Request(Macced<RequestMessage>),
}
// check at compile time that a message fits in 508 memory bytes
const _: () = [(); 1][(core::mem::size_of::<Message>() <= 508) as usize ^ 1];

// KeepAlive
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable, Copy)]
pub struct KeepAliveMessage(Timestamp);

// Init
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable, Copy)]
pub enum InitMessage {
    ConnectToQueue(Signed<(ContestId, Timestamp), PubSigKey>),
    Merkle(Signed<(ContestId, Timestamp, PubKexKey), PeerId>),
    Finalize(ContestId, PeerId, Macced<Timestamp>),
}

// Queue
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QueueMessage(Signed<QueueMessageInner, ()>);
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct QueueMessageInner {} //TODO
pub struct FileDesc {
    hash_id: u128,
    size: u32,
    encrypting_key: SizedEncrypted<EncKey, 12>,
}

// File
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct FileMessage {
    hash_id: u128,
    offset: u32,
    data: Encrypted<Vec<u8>>,
}

// Request
#[derive(PartialEq, Eq, Debug, Clone, Readable, Writable)]
pub struct RequestMessage {}
