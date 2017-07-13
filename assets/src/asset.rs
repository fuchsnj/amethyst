use std::error::Error;

use fnv::FnvHashMap;
use parking_lot::RwLock;

use StoreId;

/// One of the three core traits of this crate.
///
/// You want to implement this for every type of asset like
///
/// * `Mesh`
/// * `Texture`
/// * `Terrain`
///
/// and so on. Now, an asset may be available in different formats.
/// That's why we have the `Data` associated type here. You can specify
/// an intermediate format here, like the vertex data for a mesh or the samples
/// for audio data.
///
/// This data is then generated by the `Format` trait.
pub trait Asset
    where Self: Sized
{
    type Context;
    type Data;
    type Error: Error;

    /// A small keyword for which category this asset belongs to.
    ///
    /// ## Examples
    ///
    /// * `"mesh"` for `Mesh`
    /// * `"data"` for `Level`
    ///
    /// The storage may use this information, to e.g. search the identically-named
    /// subfolder.
    fn category() -> &'static str;

    /// Provides the conversion from the data format to the actual asset.
    fn from_data(data: Self::Data, context: &Self::Context) -> Result<Self, Self::Error>;

    /// Notifies about an asset load. This is can be used to cache the asset.
    /// To return a cached asset, see the `retrieve` function.
    fn cache(_context: &Self::Context, _spec: AssetSpec, _asset: &Self) {}

    /// Returns `Some` cached value if possible, otherwise `None`.
    ///
    /// For a basic implementation of a cache, please take a look at the `Cache` type.
    fn retrieve(_context: &Self::Context, _spec: &AssetSpec) -> Option<Self> {
        None
    }

    /// Gives a hint that several assets may have been released recently.
    ///
    /// This is useful if your assets are reference counted, because you are
    /// now able to remove unique assets from the cache, leaving the shared
    /// ones there.
    fn clear(_context: &Self::Context) {}

    /// Request for clearing the whole cache.
    fn clear_all(_context: &Self::Context) {}
}

/// A specifier for an asset, uniquely identifying it by
///
/// * the extension (the format it was provided in)
/// * it's name
/// * the storage it was loaded from
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct AssetSpec {
    /// The extension of this asset
    pub ext: &'static str,
    /// The name of this asset.
    pub name: String,
    /// The storage id, uniquely identifying the storage it's been loaded from.
    pub store: StoreId,
}

impl AssetSpec {
    /// Creates a new asset specifier from the given parameters.
    pub fn new(name: String, ext: &'static str, store: StoreId) -> Self {
        AssetSpec {
            ext,
            name,
            store,
        }
    }
}

/// A basic implementation for a cache. This might be useful as the `Context` of
/// an `Asset`, so that the same asset doesn't get imported twice.
///
/// Because contexts have to be immutable, a `RwLock` is used. Therefore, all
/// operations are blocking (but shouldn't block for a long time).
pub struct Cache<T> {
    map: RwLock<FnvHashMap<AssetSpec, T>>,
}

impl<T> Cache<T>
    where T: Clone
{
    /// Creates a new `Cache` and initializes it with the default values.
    pub fn new() -> Self {
        Default::default()
    }

    /// Inserts an asset, locking the internal `RwLock` to get write access to the hash map.
    ///
    /// Returns the previous value in case there was any.
    pub fn insert(&self, spec: AssetSpec, asset: T) -> Option<T> {
        self.map.write().insert(spec, asset)
    }

    /// Retrieves an asset, locking the internal `RwLock` to get read access to the hash map.
    /// In case this asset has been inserted previously, it will be cloned and returned.
    /// Otherwise, you'll receive `None`.
    pub fn get(&self, spec: &AssetSpec) -> Option<T> {
        self.map.read().get(spec).map(Clone::clone)
    }

    /// Deletes all cached values, except the ones `f` returned `true` for.
    /// May be used when you're about to clear unused assets (see `Asset::clear`).
    ///
    /// Blocks the calling thread for getting write access to the hash map.
    pub fn retain<F>(&self, f: F)
        where F: FnMut(&AssetSpec, &mut T) -> bool
    {
        self.map.write().retain(f);
    }

    /// Deletes all cached values after locking the `RwLock`.
    pub fn clear_all(&self) {
        self.map.write().clear();
    }
}

impl<T> Default for Cache<T> {
    fn default() -> Self {
        Cache { map: Default::default() }
    }
}

/// A format, providing a conversion from bytes to asset data, which is then
/// in turn accepted by `Asset::from_data`. Examples for formats are
/// `Png`, `Obj` and `Wave`.
pub trait Format
    where Self: Sized
{
    /// The data type this format is able to load.
    type Data;
    /// The kind of error it may produce.
    type Error: Error;

    /// Returns the extension (without `.`).
    ///
    /// ## Examples
    ///
    /// * `"png"`
    /// * `"obj"`
    /// * `"wav"`
    fn extension() -> &'static str;

    /// Reads the given bytes and produces asset data.
    fn parse(&self, bytes: Vec<u8>) -> Result<Self::Data, Self::Error>;
}
