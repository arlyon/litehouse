// automatically generated by the FlatBuffers compiler, do not modify


// @generated

use core::mem;
use core::cmp::Ordering;

extern crate flatbuffers;
use self::flatbuffers::{EndianScalar, Follow};

#[allow(unused_imports, dead_code)]
pub mod litehouse {

  use core::mem;
  use core::cmp::Ordering;

  extern crate flatbuffers;
  use self::flatbuffers::{EndianScalar, Follow};

// struct Version, aligned to 2
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq)]
pub struct Version(pub [u8; 6]);
impl Default for Version { 
  fn default() -> Self { 
    Self([0; 6])
  }
}
impl core::fmt::Debug for Version {
  fn fmt(&self, f: &mut core::fmt::Formatter) -> core::fmt::Result {
    f.debug_struct("Version")
      .field("major", &self.major())
      .field("minor", &self.minor())
      .field("patch", &self.patch())
      .finish()
  }
}

impl flatbuffers::SimpleToVerifyInSlice for Version {}
impl<'a> flatbuffers::Follow<'a> for Version {
  type Inner = &'a Version;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    <&'a Version>::follow(buf, loc)
  }
}
impl<'a> flatbuffers::Follow<'a> for &'a Version {
  type Inner = &'a Version;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    flatbuffers::follow_cast_ref::<Version>(buf, loc)
  }
}
impl<'b> flatbuffers::Push for Version {
    type Output = Version;
    #[inline]
    unsafe fn push(&self, dst: &mut [u8], _written_len: usize) {
        let src = ::core::slice::from_raw_parts(self as *const Version as *const u8, Self::size());
        dst.copy_from_slice(src);
    }
}

impl<'a> flatbuffers::Verifiable for Version {
  #[inline]
  fn run_verifier(
    v: &mut flatbuffers::Verifier, pos: usize
  ) -> Result<(), flatbuffers::InvalidFlatbuffer> {
    use self::flatbuffers::Verifiable;
    v.in_buffer::<Self>(pos)
  }
}

impl<'a> Version {
  #[allow(clippy::too_many_arguments)]
  pub fn new(
    major: u16,
    minor: u16,
    patch: u16,
  ) -> Self {
    let mut s = Self([0; 6]);
    s.set_major(major);
    s.set_minor(minor);
    s.set_patch(patch);
    s
  }

  pub fn major(&self) -> u16 {
    let mut mem = core::mem::MaybeUninit::<<u16 as EndianScalar>::Scalar>::uninit();
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid value in this slot
    EndianScalar::from_little_endian(unsafe {
      core::ptr::copy_nonoverlapping(
        self.0[0..].as_ptr(),
        mem.as_mut_ptr() as *mut u8,
        core::mem::size_of::<<u16 as EndianScalar>::Scalar>(),
      );
      mem.assume_init()
    })
  }

  pub fn set_major(&mut self, x: u16) {
    let x_le = x.to_little_endian();
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid value in this slot
    unsafe {
      core::ptr::copy_nonoverlapping(
        &x_le as *const _ as *const u8,
        self.0[0..].as_mut_ptr(),
        core::mem::size_of::<<u16 as EndianScalar>::Scalar>(),
      );
    }
  }

  pub fn minor(&self) -> u16 {
    let mut mem = core::mem::MaybeUninit::<<u16 as EndianScalar>::Scalar>::uninit();
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid value in this slot
    EndianScalar::from_little_endian(unsafe {
      core::ptr::copy_nonoverlapping(
        self.0[2..].as_ptr(),
        mem.as_mut_ptr() as *mut u8,
        core::mem::size_of::<<u16 as EndianScalar>::Scalar>(),
      );
      mem.assume_init()
    })
  }

  pub fn set_minor(&mut self, x: u16) {
    let x_le = x.to_little_endian();
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid value in this slot
    unsafe {
      core::ptr::copy_nonoverlapping(
        &x_le as *const _ as *const u8,
        self.0[2..].as_mut_ptr(),
        core::mem::size_of::<<u16 as EndianScalar>::Scalar>(),
      );
    }
  }

  pub fn patch(&self) -> u16 {
    let mut mem = core::mem::MaybeUninit::<<u16 as EndianScalar>::Scalar>::uninit();
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid value in this slot
    EndianScalar::from_little_endian(unsafe {
      core::ptr::copy_nonoverlapping(
        self.0[4..].as_ptr(),
        mem.as_mut_ptr() as *mut u8,
        core::mem::size_of::<<u16 as EndianScalar>::Scalar>(),
      );
      mem.assume_init()
    })
  }

  pub fn set_patch(&mut self, x: u16) {
    let x_le = x.to_little_endian();
    // Safety:
    // Created from a valid Table for this object
    // Which contains a valid value in this slot
    unsafe {
      core::ptr::copy_nonoverlapping(
        &x_le as *const _ as *const u8,
        self.0[4..].as_mut_ptr(),
        core::mem::size_of::<<u16 as EndianScalar>::Scalar>(),
      );
    }
  }

}

pub enum SummariesOffset {}
#[derive(Copy, Clone, PartialEq)]

pub struct Summaries<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Summaries<'a> {
  type Inner = Summaries<'a>;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    Self { _tab: flatbuffers::Table::new(buf, loc) }
  }
}

impl<'a> Summaries<'a> {
  pub const VT_SUMMARIES: flatbuffers::VOffsetT = 4;

  #[inline]
  pub unsafe fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
    Summaries { _tab: table }
  }
  #[allow(unused_mut)]
  pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
    _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
    args: &'args SummariesArgs<'args>
  ) -> flatbuffers::WIPOffset<Summaries<'bldr>> {
    let mut builder = SummariesBuilder::new(_fbb);
    if let Some(x) = args.summaries { builder.add_summaries(x); }
    builder.finish()
  }


  #[inline]
  pub fn summaries(&self) -> Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<Summary<'a>>>> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<Summary>>>>(Summaries::VT_SUMMARIES, None)}
  }
}

impl flatbuffers::Verifiable for Summaries<'_> {
  #[inline]
  fn run_verifier(
    v: &mut flatbuffers::Verifier, pos: usize
  ) -> Result<(), flatbuffers::InvalidFlatbuffer> {
    use self::flatbuffers::Verifiable;
    v.visit_table(pos)?
     .visit_field::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<Summary>>>>("summaries", Self::VT_SUMMARIES, false)?
     .finish();
    Ok(())
  }
}
pub struct SummariesArgs<'a> {
    pub summaries: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<Summary<'a>>>>>,
}
impl<'a> Default for SummariesArgs<'a> {
  #[inline]
  fn default() -> Self {
    SummariesArgs {
      summaries: None,
    }
  }
}

pub struct SummariesBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> SummariesBuilder<'a, 'b> {
  #[inline]
  pub fn add_summaries(&mut self, summaries: flatbuffers::WIPOffset<flatbuffers::Vector<'b , flatbuffers::ForwardsUOffset<Summary<'b >>>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Summaries::VT_SUMMARIES, summaries);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> SummariesBuilder<'a, 'b> {
    let start = _fbb.start_table();
    SummariesBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<Summaries<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

impl core::fmt::Debug for Summaries<'_> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut ds = f.debug_struct("Summaries");
      ds.field("summaries", &self.summaries());
      ds.finish()
  }
}
pub enum SummaryOffset {}
#[derive(Copy, Clone, PartialEq)]

pub struct Summary<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Summary<'a> {
  type Inner = Summary<'a>;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    Self { _tab: flatbuffers::Table::new(buf, loc) }
  }
}

impl<'a> Summary<'a> {
  pub const VT_TITLE: flatbuffers::VOffsetT = 4;
  pub const VT_VERSIONS: flatbuffers::VOffsetT = 6;
  pub const VT_SIZE_: flatbuffers::VOffsetT = 8;
  pub const VT_DESCRIPTION: flatbuffers::VOffsetT = 10;

  #[inline]
  pub unsafe fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
    Summary { _tab: table }
  }
  #[allow(unused_mut)]
  pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
    _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
    args: &'args SummaryArgs<'args>
  ) -> flatbuffers::WIPOffset<Summary<'bldr>> {
    let mut builder = SummaryBuilder::new(_fbb);
    if let Some(x) = args.description { builder.add_description(x); }
    builder.add_size_(args.size_);
    if let Some(x) = args.versions { builder.add_versions(x); }
    if let Some(x) = args.title { builder.add_title(x); }
    builder.finish()
  }


  #[inline]
  pub fn title(&self) -> Option<&'a str> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Summary::VT_TITLE, None)}
  }
  #[inline]
  pub fn versions(&self) -> Option<flatbuffers::Vector<'a, Version>> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, Version>>>(Summary::VT_VERSIONS, None)}
  }
  #[inline]
  pub fn size_(&self) -> u32 {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<u32>(Summary::VT_SIZE_, Some(0)).unwrap()}
  }
  #[inline]
  pub fn description(&self) -> Option<&'a str> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Summary::VT_DESCRIPTION, None)}
  }
}

impl flatbuffers::Verifiable for Summary<'_> {
  #[inline]
  fn run_verifier(
    v: &mut flatbuffers::Verifier, pos: usize
  ) -> Result<(), flatbuffers::InvalidFlatbuffer> {
    use self::flatbuffers::Verifiable;
    v.visit_table(pos)?
     .visit_field::<flatbuffers::ForwardsUOffset<&str>>("title", Self::VT_TITLE, false)?
     .visit_field::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'_, Version>>>("versions", Self::VT_VERSIONS, false)?
     .visit_field::<u32>("size_", Self::VT_SIZE_, false)?
     .visit_field::<flatbuffers::ForwardsUOffset<&str>>("description", Self::VT_DESCRIPTION, false)?
     .finish();
    Ok(())
  }
}
pub struct SummaryArgs<'a> {
    pub title: Option<flatbuffers::WIPOffset<&'a str>>,
    pub versions: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a, Version>>>,
    pub size_: u32,
    pub description: Option<flatbuffers::WIPOffset<&'a str>>,
}
impl<'a> Default for SummaryArgs<'a> {
  #[inline]
  fn default() -> Self {
    SummaryArgs {
      title: None,
      versions: None,
      size_: 0,
      description: None,
    }
  }
}

pub struct SummaryBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> SummaryBuilder<'a, 'b> {
  #[inline]
  pub fn add_title(&mut self, title: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Summary::VT_TITLE, title);
  }
  #[inline]
  pub fn add_versions(&mut self, versions: flatbuffers::WIPOffset<flatbuffers::Vector<'b , Version>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Summary::VT_VERSIONS, versions);
  }
  #[inline]
  pub fn add_size_(&mut self, size_: u32) {
    self.fbb_.push_slot::<u32>(Summary::VT_SIZE_, size_, 0);
  }
  #[inline]
  pub fn add_description(&mut self, description: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Summary::VT_DESCRIPTION, description);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> SummaryBuilder<'a, 'b> {
    let start = _fbb.start_table();
    SummaryBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<Summary<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

impl core::fmt::Debug for Summary<'_> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut ds = f.debug_struct("Summary");
      ds.field("title", &self.title());
      ds.field("versions", &self.versions());
      ds.field("size_", &self.size_());
      ds.field("description", &self.description());
      ds.finish()
  }
}
pub enum EntryOffset {}
#[derive(Copy, Clone, PartialEq)]

pub struct Entry<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Entry<'a> {
  type Inner = Entry<'a>;
  #[inline]
  unsafe fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    Self { _tab: flatbuffers::Table::new(buf, loc) }
  }
}

impl<'a> Entry<'a> {
  pub const VT_TITLE: flatbuffers::VOffsetT = 4;
  pub const VT_VERSION: flatbuffers::VOffsetT = 6;
  pub const VT_DESCRIPTION: flatbuffers::VOffsetT = 8;
  pub const VT_CAPABILITIES: flatbuffers::VOffsetT = 10;
  pub const VT_SCHEMA: flatbuffers::VOffsetT = 12;
  pub const VT_SIZE_: flatbuffers::VOffsetT = 14;
  pub const VT_SHA: flatbuffers::VOffsetT = 16;

  #[inline]
  pub unsafe fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
    Entry { _tab: table }
  }
  #[allow(unused_mut)]
  pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
    _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
    args: &'args EntryArgs<'args>
  ) -> flatbuffers::WIPOffset<Entry<'bldr>> {
    let mut builder = EntryBuilder::new(_fbb);
    if let Some(x) = args.sha { builder.add_sha(x); }
    builder.add_size_(args.size_);
    if let Some(x) = args.schema { builder.add_schema(x); }
    if let Some(x) = args.capabilities { builder.add_capabilities(x); }
    if let Some(x) = args.description { builder.add_description(x); }
    if let Some(x) = args.version { builder.add_version(x); }
    if let Some(x) = args.title { builder.add_title(x); }
    builder.finish()
  }


  #[inline]
  pub fn title(&self) -> Option<&'a str> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Entry::VT_TITLE, None)}
  }
  #[inline]
  pub fn version(&self) -> Option<&'a Version> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<Version>(Entry::VT_VERSION, None)}
  }
  #[inline]
  pub fn description(&self) -> Option<&'a str> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Entry::VT_DESCRIPTION, None)}
  }
  #[inline]
  pub fn capabilities(&self) -> Option<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>>(Entry::VT_CAPABILITIES, None)}
  }
  #[inline]
  pub fn schema(&self) -> Option<&'a str> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Entry::VT_SCHEMA, None)}
  }
  #[inline]
  pub fn size_(&self) -> u32 {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<u32>(Entry::VT_SIZE_, Some(0)).unwrap()}
  }
  #[inline]
  pub fn sha(&self) -> Option<&'a str> {
    // Safety:
    // Created from valid Table for this object
    // which contains a valid value in this slot
    unsafe { self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Entry::VT_SHA, None)}
  }
}

impl flatbuffers::Verifiable for Entry<'_> {
  #[inline]
  fn run_verifier(
    v: &mut flatbuffers::Verifier, pos: usize
  ) -> Result<(), flatbuffers::InvalidFlatbuffer> {
    use self::flatbuffers::Verifiable;
    v.visit_table(pos)?
     .visit_field::<flatbuffers::ForwardsUOffset<&str>>("title", Self::VT_TITLE, false)?
     .visit_field::<Version>("version", Self::VT_VERSION, false)?
     .visit_field::<flatbuffers::ForwardsUOffset<&str>>("description", Self::VT_DESCRIPTION, false)?
     .visit_field::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'_, flatbuffers::ForwardsUOffset<&'_ str>>>>("capabilities", Self::VT_CAPABILITIES, false)?
     .visit_field::<flatbuffers::ForwardsUOffset<&str>>("schema", Self::VT_SCHEMA, false)?
     .visit_field::<u32>("size_", Self::VT_SIZE_, false)?
     .visit_field::<flatbuffers::ForwardsUOffset<&str>>("sha", Self::VT_SHA, false)?
     .finish();
    Ok(())
  }
}
pub struct EntryArgs<'a> {
    pub title: Option<flatbuffers::WIPOffset<&'a str>>,
    pub version: Option<&'a Version>,
    pub description: Option<flatbuffers::WIPOffset<&'a str>>,
    pub capabilities: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a, flatbuffers::ForwardsUOffset<&'a str>>>>,
    pub schema: Option<flatbuffers::WIPOffset<&'a str>>,
    pub size_: u32,
    pub sha: Option<flatbuffers::WIPOffset<&'a str>>,
}
impl<'a> Default for EntryArgs<'a> {
  #[inline]
  fn default() -> Self {
    EntryArgs {
      title: None,
      version: None,
      description: None,
      capabilities: None,
      schema: None,
      size_: 0,
      sha: None,
    }
  }
}

pub struct EntryBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> EntryBuilder<'a, 'b> {
  #[inline]
  pub fn add_title(&mut self, title: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Entry::VT_TITLE, title);
  }
  #[inline]
  pub fn add_version(&mut self, version: &Version) {
    self.fbb_.push_slot_always::<&Version>(Entry::VT_VERSION, version);
  }
  #[inline]
  pub fn add_description(&mut self, description: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Entry::VT_DESCRIPTION, description);
  }
  #[inline]
  pub fn add_capabilities(&mut self, capabilities: flatbuffers::WIPOffset<flatbuffers::Vector<'b , flatbuffers::ForwardsUOffset<&'b  str>>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Entry::VT_CAPABILITIES, capabilities);
  }
  #[inline]
  pub fn add_schema(&mut self, schema: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Entry::VT_SCHEMA, schema);
  }
  #[inline]
  pub fn add_size_(&mut self, size_: u32) {
    self.fbb_.push_slot::<u32>(Entry::VT_SIZE_, size_, 0);
  }
  #[inline]
  pub fn add_sha(&mut self, sha: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Entry::VT_SHA, sha);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> EntryBuilder<'a, 'b> {
    let start = _fbb.start_table();
    EntryBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<Entry<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

impl core::fmt::Debug for Entry<'_> {
  fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
    let mut ds = f.debug_struct("Entry");
      ds.field("title", &self.title());
      ds.field("version", &self.version());
      ds.field("description", &self.description());
      ds.field("capabilities", &self.capabilities());
      ds.field("schema", &self.schema());
      ds.field("size_", &self.size_());
      ds.field("sha", &self.sha());
      ds.finish()
  }
}
}  // pub mod litehouse

