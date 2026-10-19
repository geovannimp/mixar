//! DeviceSQL `export.pdb` writer.
//!
//! The database is a paged store: a 4096-byte file header (page 0) listing the
//! table pointers, then per-table index ("header") pages and data pages. Rows
//! are packed into a page heap growing up from `0x28` with a row-offset footer
//! growing down from the page end; multi-byte fields are little-endian.
//!
//! Tables written: tracks, genres, artists, albums, labels, keys, colors,
//! playlist tree, playlist entries, artwork, columns, menu, table `0x12`,
//! history playlists, history entries and history. The tables the hardware
//! browses without rows (labels, history playlists/entries, the unknown tables
//! `0x09`/`0x0A`/`0x0E`/`0x0F`, and artwork when the export has no covers) are
//! written as index pages only; `keys`, `colors`, `columns`, `menu`, `0x12` and
//! `history` carry the fixed rows the schema expects, and artwork carries one
//! row per distinct cover.
//!
//! # Provenance
//!
//! The page layout, the fixed "magic" field values and the constant tables are
//! the ones real CDJ/XDJ hardware accepts. Sources, in the order the details
//! were checked:
//!
//! - a hardware-validated (XDJ-XZ) reference exporter, `sustain`
//!   (`crates/pioneer/src/pdb.rs`, GPL-3.0-or-later, © 2026
//!   AnnoyingTechnology) — the page allocator, the fixed `COLUMNS`/`MENU_ROWS`
//!   /`UNKNOWN_18_ROWS` schemas and the per-table row serializers are adapted
//!   from it;
//! - the *fourfour* `pioneer-usb-writer` reverse-engineering notes validated on
//!   CDJ-3000 firmware 3.19 (page-0 fields, sequence bases, the Tracks/History
//!   index entries, `RowTooLarge` sizing);
//! - `rekordcrate` (MPL-2.0) — the independent parser used by the tests here —
//!   and two Rekordbox-written exports (an empty one and a seven-track one)
//!   inspected while writing this module.
//!
//! # Deviations from the reference exporter
//!
//! - **Artwork rows.** Each row is an id plus the DeviceSQL path inlined into
//!   the row (no offset indirection), padded to the reference exporter's
//!   36-byte stride; the table keeps the fixed page after its index page as its
//!   first data page and takes a fresh empty-candidate page whenever it has
//!   rows. Without covers the pointer stays at Rekordbox's empty shape
//!   (`empty = 28`, `last = 27`).
//! - **Real Rekordbox fixed tables.** The `columns` labels, the 22-row browse
//!   menu and the 17-row table `0x12` are the rows the two Rekordbox exports
//!   above carry (byte-identical between the two), not the reference's
//!   differently ordered variants.
//! - **Packed row counts.** Bytes `0x18..0x1B` of every page are the 13-bit
//!   total and 11-bit valid row counts Rekordbox actually writes (read by the
//!   current `rekordcrate` as `PackedRowCounts`). The reverse-engineering notes
//!   describe the same bytes as `num_rows_small` / `unk3 = (n % 8) * 0x20` /
//!   `unk4 = ceil(n / 16)`; that reading coincides with the packed field for up
//!   to 15 rows per page and diverges above. The rows here are all present, so
//!   the valid count equals the total in every page this writer emits.
//! - **Tracks/History index entries.** Their index pages list their data pages
//!   (`num_entries`, the `0x1FFF000N` word and the entries themselves), which
//!   is what Rekordbox writes and what CDJ-3000 firmware 3.19 needs; the other
//!   tables leave their index pages empty as Rekordbox does. The reference
//!   exporter left every index page empty.
//! - Every table that can hold rows gets a data page even when it has none,
//!   which is the reference's shape; Rekordbox omits the empty page and points
//!   the table's `last_page` back at its index page instead. All page pointers
//!   here are inside the file and self-consistent either way.
//! - No track/disc numbers: Mixar's model has none, so those fields stay zero,
//!   and neither is the track's colour id set. Rating, comment, colour, year and
//!   track/disc numbers are tracked in
//!   [#571](https://github.com/geovannimp/mixar/issues/571).

use std::collections::HashMap;
use std::ops::Range;

use super::device_sql::encode;
use super::keys::KEY_TABLE;
use super::model::{CdjArtwork, CdjPlaylist, CdjTrack};

/// Size of one database page.
const PAGE_SIZE: usize = 4096;
/// Offset of the page heap, right after the 0x28-byte page header.
const HEAP_START: usize = 0x28;
/// Number of table pointers in page 0.
const NUM_TABLES: u32 = 20;
/// Pages 0–40 hold every table when nothing overflows.
const STANDARD_PAGES: usize = 41;
/// First page past the reserved empty-candidate zone (pages 41–52); overflow
/// data pages are allocated from here up.
const OVERFLOW_START: u32 = 53;
/// Rows per data page. Keeps the 13-bit row count and the packed
/// `num_rows_small` byte unambiguous, and Rekordbox never fills a page further.
const MAX_ROWS_PER_PAGE: usize = 255;
/// "No page" marker used in index-page content.
const NO_PAGE: u32 = 0x03FF_FFFF;

// Table type discriminants (DeviceSQL table ids).
const T_TRACKS: u32 = 0x00;
const T_GENRES: u32 = 0x01;
const T_ARTISTS: u32 = 0x02;
const T_ALBUMS: u32 = 0x03;
const T_LABELS: u32 = 0x04;
const T_KEYS: u32 = 0x05;
const T_COLORS: u32 = 0x06;
const T_PLAYLIST_TREE: u32 = 0x07;
const T_PLAYLIST_ENTRIES: u32 = 0x08;
const T_UNKNOWN_09: u32 = 0x09;
const T_UNKNOWN_0A: u32 = 0x0A;
const T_HISTORY_PLAYLISTS: u32 = 0x0B;
const T_HISTORY_ENTRIES: u32 = 0x0C;
const T_ARTWORK: u32 = 0x0D;
const T_UNKNOWN_0E: u32 = 0x0E;
const T_UNKNOWN_0F: u32 = 0x0F;
const T_COLUMNS: u32 = 0x10;
const T_MENU: u32 = 0x11;
const T_UNKNOWN_12: u32 = 0x12;
const T_HISTORY: u32 = 0x13;

// Fixed page layouts (see the reference exporter and both Rekordbox exports).
const TRACKS_INDEX_PAGE: u32 = 1;
const TRACKS_FIRST_DATA_PAGE: u32 = 2;
const GENRES_INDEX_PAGE: u32 = 3;
const GENRES_FIRST_DATA_PAGE: u32 = 4;
const ARTISTS_INDEX_PAGE: u32 = 5;
const ARTISTS_FIRST_DATA_PAGE: u32 = 6;
const ALBUMS_INDEX_PAGE: u32 = 7;
const ALBUMS_FIRST_DATA_PAGE: u32 = 8;
const KEYS_INDEX_PAGE: u32 = 11;
const KEYS_DATA_PAGE: u32 = 12;
const COLORS_INDEX_PAGE: u32 = 13;
const COLORS_DATA_PAGE: u32 = 14;
const TREE_INDEX_PAGE: u32 = 15;
const TREE_FIRST_DATA_PAGE: u32 = 16;
const ENTRIES_INDEX_PAGE: u32 = 17;
const ENTRIES_FIRST_DATA_PAGE: u32 = 18;
const ARTWORK_INDEX_PAGE: u32 = 27;
const ARTWORK_FIRST_DATA_PAGE: u32 = 28;
const COLUMNS_INDEX_PAGE: u32 = 33;
const COLUMNS_DATA_PAGE: u32 = 34;
const MENU_INDEX_PAGE: u32 = 35;
const MENU_DATA_PAGE: u32 = 36;
const UNKNOWN_12_INDEX_PAGE: u32 = 37;
const UNKNOWN_12_DATA_PAGE: u32 = 38;
const HISTORY_INDEX_PAGE: u32 = 39;
const HISTORY_DATA_PAGE: u32 = 40;

// Reserved empty-candidate pages: the page a table would grow into next. Fixed
// tables and the reference's per-table reservations, all inside the 41–52 zone
// that Rekordbox leaves zeroed.
const HISTORY_EMPTY_PAGE: u32 = 41;
const COLORS_EMPTY_PAGE: u32 = 42;
const COLUMNS_EMPTY_PAGE: u32 = 43;
const MENU_EMPTY_PAGE: u32 = 44;
const UNKNOWN_12_EMPTY_PAGE: u32 = 45;
const TREE_EMPTY_PAGE: u32 = 46;
const ARTISTS_EMPTY_PAGE: u32 = 47;
const GENRES_EMPTY_PAGE: u32 = 48;
const ALBUMS_EMPTY_PAGE: u32 = 49;
const KEYS_EMPTY_PAGE: u32 = 50;
const TRACKS_EMPTY_PAGE: u32 = 51;
const ENTRIES_EMPTY_PAGE: u32 = 52;

/// Failure while building the device database.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PdbError {
    /// A row does not fit on a page: pathological metadata (a single string
    /// longer than a whole page can address).
    RowTooLarge {
        /// Table the oversized row belongs to.
        table: &'static str,
        /// Serialized row size in bytes.
        bytes: usize,
    },
}

impl std::fmt::Display for PdbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::RowTooLarge { table, bytes } => write!(
                f,
                "{table} row of {bytes} bytes does not fit in a device database page"
            ),
        }
    }
}

impl std::error::Error for PdbError {}

/// Build the complete `export.pdb` image for `tracks` and `playlists`.
///
/// `export_date` is the `YYYY-MM-DD` string stamped into every track's
/// analysis-date field and into the history row. `CdjPlaylist::entries` index
/// `tracks` in play order; artists, albums and genres are interned in
/// first-appearance order, so the same input always produces the same bytes.
pub fn build(
    tracks: &[CdjTrack],
    playlists: &[CdjPlaylist],
    artworks: &[CdjArtwork],
    export_date: &str,
) -> Result<Vec<u8>, PdbError> {
    let entities = Entities::build(tracks);

    // Serialize every row up front: their sizes drive page chunking.
    let mut track_rows: Vec<Vec<u8>> = tracks
        .iter()
        .enumerate()
        .map(|(index, track)| track_row(track, &entities, index, export_date))
        .collect();
    let mut artist_rows: Vec<Vec<u8>> = entities
        .artists
        .iter()
        .enumerate()
        .map(|(index, name)| artist_row(name, index + 1))
        .collect();
    let mut album_rows: Vec<Vec<u8>> = entities
        .albums
        .iter()
        .enumerate()
        .map(|(index, name)| album_row(name, index + 1))
        .collect();
    let mut genre_rows: Vec<Vec<u8>> = entities
        .genres
        .iter()
        .enumerate()
        .map(|(index, name)| genre_row(name, index + 1))
        .collect();
    let mut tree_rows: Vec<Vec<u8>> = playlists
        .iter()
        .enumerate()
        .map(|(index, playlist)| playlist_tree_row(&playlist.name, index))
        .collect();
    let mut entry_rows = playlist_entry_rows(playlists);
    let mut artwork_rows: Vec<Vec<u8>> = artworks.iter().map(artwork_row).collect();

    let track_chunks = chunk_rows("tracks", &track_rows)?;
    let artist_chunks = chunk_rows("artists", &artist_rows)?;
    let album_chunks = chunk_rows("albums", &album_rows)?;
    let genre_chunks = chunk_rows("genres", &genre_rows)?;
    let tree_chunks = chunk_rows("playlist tree", &tree_rows)?;
    let entry_chunks = chunk_rows("playlist entries", &entry_rows)?;
    let artwork_chunks = chunk_rows("artwork", &artwork_rows)?;

    // Allocate page numbers: the fixed first pages, then a monotonic overflow
    // counter for tables that need more than one data page.
    let mut next = OVERFLOW_START;
    let tracks_alloc = assign(
        "tracks",
        &track_chunks,
        TRACKS_FIRST_DATA_PAGE,
        TRACKS_EMPTY_PAGE,
        &mut next,
    );
    let genres_alloc = assign(
        "genres",
        &genre_chunks,
        GENRES_FIRST_DATA_PAGE,
        GENRES_EMPTY_PAGE,
        &mut next,
    );
    let artists_alloc = assign(
        "artists",
        &artist_chunks,
        ARTISTS_FIRST_DATA_PAGE,
        ARTISTS_EMPTY_PAGE,
        &mut next,
    );
    let albums_alloc = assign(
        "albums",
        &album_chunks,
        ALBUMS_FIRST_DATA_PAGE,
        ALBUMS_EMPTY_PAGE,
        &mut next,
    );
    let tree_alloc = assign(
        "playlist tree",
        &tree_chunks,
        TREE_FIRST_DATA_PAGE,
        TREE_EMPTY_PAGE,
        &mut next,
    );
    let entries_alloc = assign(
        "playlist entries",
        &entry_chunks,
        ENTRIES_FIRST_DATA_PAGE,
        ENTRIES_EMPTY_PAGE,
        &mut next,
    );
    // Artwork grows from the fixed page after its index page; when it has rows
    // the empty candidate is the next fresh page (the reference exporter's
    // shape), and without rows the table keeps its index-only pointer.
    let artwork_alloc = if artwork_rows.is_empty() {
        None
    } else {
        Some(assign_artwork(&artwork_chunks, &mut next))
    };

    let num_pages = if next > OVERFLOW_START {
        next as usize
    } else {
        STANDARD_PAGES
    };
    let mut file = FileBuf::new(num_pages);

    // --- Tables whose rows come from the input, in page-0 pointer order ---
    let mut highest_sequence = 0;

    // The Tracks and History index pages list their data pages for the
    // hardware's benefit; every other table leaves its index page empty, as
    // Rekordbox does.
    write_index_page(
        &mut file,
        TRACKS_INDEX_PAGE,
        T_TRACKS,
        tracks_alloc.first(),
        &tracks_alloc.data_pages,
    );
    highest_sequence = highest_sequence.max(write_rows_table(
        &mut file,
        "tracks",
        T_TRACKS,
        &tracks_alloc,
        &track_chunks,
        &mut track_rows,
        TableCfg {
            base_seq: 10,
            seq_step: 5,
            plus_one_at_11: true,
            index_shift: true,
        },
    )?);

    write_index_page(
        &mut file,
        GENRES_INDEX_PAGE,
        T_GENRES,
        genres_alloc.first(),
        &[],
    );
    highest_sequence = highest_sequence.max(write_rows_table(
        &mut file,
        "genres",
        T_GENRES,
        &genres_alloc,
        &genre_chunks,
        &mut genre_rows,
        TableCfg {
            base_seq: 8,
            seq_step: 5,
            plus_one_at_11: true,
            index_shift: false,
        },
    )?);

    write_index_page(
        &mut file,
        ARTISTS_INDEX_PAGE,
        T_ARTISTS,
        artists_alloc.first(),
        &[],
    );
    highest_sequence = highest_sequence.max(write_rows_table(
        &mut file,
        "artists",
        T_ARTISTS,
        &artists_alloc,
        &artist_chunks,
        &mut artist_rows,
        TableCfg {
            base_seq: 7,
            seq_step: 5,
            plus_one_at_11: true,
            index_shift: true,
        },
    )?);

    write_index_page(
        &mut file,
        ALBUMS_INDEX_PAGE,
        T_ALBUMS,
        albums_alloc.first(),
        &[],
    );
    highest_sequence = highest_sequence.max(write_rows_table(
        &mut file,
        "albums",
        T_ALBUMS,
        &albums_alloc,
        &album_chunks,
        &mut album_rows,
        TableCfg {
            base_seq: 9,
            seq_step: 5,
            plus_one_at_11: true,
            index_shift: true,
        },
    )?);

    // --- Labels (0x04): no rows in a Mixar export ---
    write_index_page(&mut file, 9, T_LABELS, 10, &[]);

    // --- Keys (0x05): the fixed 24 rows ---
    write_index_page(&mut file, KEYS_INDEX_PAGE, T_KEYS, KEYS_DATA_PAGE, &[]);
    highest_sequence = highest_sequence.max(write_keys_page(&mut file, tracks.len())?);

    // --- Colors (0x06): the fixed eight color labels ---
    write_index_page(
        &mut file,
        COLORS_INDEX_PAGE,
        T_COLORS,
        COLORS_DATA_PAGE,
        &[],
    );
    highest_sequence = highest_sequence.max(write_colors_page(&mut file)?);

    // --- Playlist tree (0x07) and entries (0x08) ---
    write_index_page(
        &mut file,
        TREE_INDEX_PAGE,
        T_PLAYLIST_TREE,
        tree_alloc.first(),
        &[],
    );
    highest_sequence = highest_sequence.max(write_rows_table(
        &mut file,
        "playlist tree",
        T_PLAYLIST_TREE,
        &tree_alloc,
        &tree_chunks,
        &mut tree_rows,
        TableCfg {
            base_seq: 6,
            seq_step: 1,
            plus_one_at_11: false,
            index_shift: false,
        },
    )?);

    write_index_page(
        &mut file,
        ENTRIES_INDEX_PAGE,
        T_PLAYLIST_ENTRIES,
        entries_alloc.first(),
        &[],
    );
    highest_sequence = highest_sequence.max(write_rows_table(
        &mut file,
        "playlist entries",
        T_PLAYLIST_ENTRIES,
        &entries_alloc,
        &entry_chunks,
        &mut entry_rows,
        TableCfg {
            base_seq: 11,
            seq_step: 5,
            plus_one_at_11: true,
            index_shift: false,
        },
    )?);

    // --- Artwork (0x0D): one row per distinct cover, one file per row ---
    match &artwork_alloc {
        Some(alloc) => {
            write_index_page(&mut file, ARTWORK_INDEX_PAGE, T_ARTWORK, alloc.first(), &[]);
            highest_sequence = highest_sequence.max(write_rows_table(
                &mut file,
                "artwork",
                T_ARTWORK,
                alloc,
                &artwork_chunks,
                &mut artwork_rows,
                TableCfg {
                    base_seq: 8,
                    seq_step: 5,
                    plus_one_at_11: false,
                    index_shift: false,
                },
            )?);
        }
        None => write_index_page(
            &mut file,
            ARTWORK_INDEX_PAGE,
            T_ARTWORK,
            ARTWORK_FIRST_DATA_PAGE,
            &[],
        ),
    }

    // --- Tables the hardware requires without rows for this export ---
    for (index_page, table_type) in [
        (19, T_UNKNOWN_09),
        (21, T_UNKNOWN_0A),
        (23, T_HISTORY_PLAYLISTS),
        (25, T_HISTORY_ENTRIES),
        (29, T_UNKNOWN_0E),
        (31, T_UNKNOWN_0F),
    ] {
        write_index_page(&mut file, index_page, table_type, index_page + 1, &[]);
    }

    // --- Columns (0x10), menu (0x11) and table 0x12: fixed schemas ---
    write_index_page(
        &mut file,
        COLUMNS_INDEX_PAGE,
        T_COLUMNS,
        COLUMNS_DATA_PAGE,
        &[],
    );
    highest_sequence = highest_sequence.max(write_columns_page(&mut file)?);

    write_index_page(&mut file, MENU_INDEX_PAGE, T_MENU, MENU_DATA_PAGE, &[]);
    highest_sequence = highest_sequence.max(write_menu_page(&mut file)?);

    write_index_page(
        &mut file,
        UNKNOWN_12_INDEX_PAGE,
        T_UNKNOWN_12,
        UNKNOWN_12_DATA_PAGE,
        &[],
    );
    highest_sequence = highest_sequence.max(write_unknown_12_page(&mut file)?);

    // --- History (0x13): a database without history rows is rejected ---
    write_index_page(
        &mut file,
        HISTORY_INDEX_PAGE,
        T_HISTORY,
        HISTORY_DATA_PAGE,
        &[HISTORY_DATA_PAGE],
    );
    highest_sequence =
        highest_sequence.max(write_history_page(&mut file, tracks.len(), export_date)?);

    // --- Page 0: file header and table pointers ---
    let pointers = [
        (
            T_TRACKS,
            tracks_alloc.empty,
            TRACKS_INDEX_PAGE,
            tracks_alloc.last,
        ),
        (
            T_GENRES,
            genres_alloc.empty,
            GENRES_INDEX_PAGE,
            genres_alloc.last,
        ),
        (
            T_ARTISTS,
            artists_alloc.empty,
            ARTISTS_INDEX_PAGE,
            artists_alloc.last,
        ),
        (
            T_ALBUMS,
            albums_alloc.empty,
            ALBUMS_INDEX_PAGE,
            albums_alloc.last,
        ),
        (T_LABELS, 10, 9, 9),
        (T_KEYS, KEYS_EMPTY_PAGE, KEYS_INDEX_PAGE, KEYS_DATA_PAGE),
        (
            T_COLORS,
            COLORS_EMPTY_PAGE,
            COLORS_INDEX_PAGE,
            COLORS_DATA_PAGE,
        ),
        (
            T_PLAYLIST_TREE,
            tree_alloc.empty,
            TREE_INDEX_PAGE,
            tree_alloc.last,
        ),
        (
            T_PLAYLIST_ENTRIES,
            entries_alloc.empty,
            ENTRIES_INDEX_PAGE,
            entries_alloc.last,
        ),
        (T_UNKNOWN_09, 20, 19, 19),
        (T_UNKNOWN_0A, 22, 21, 21),
        (T_HISTORY_PLAYLISTS, 24, 23, 23),
        (T_HISTORY_ENTRIES, 26, 25, 25),
        (
            T_ARTWORK,
            artwork_alloc
                .as_ref()
                .map_or(ARTWORK_FIRST_DATA_PAGE, |alloc| alloc.empty),
            ARTWORK_INDEX_PAGE,
            artwork_alloc
                .as_ref()
                .map_or(ARTWORK_INDEX_PAGE, |alloc| alloc.last),
        ),
        (T_UNKNOWN_0E, 30, 29, 29),
        (T_UNKNOWN_0F, 32, 31, 31),
        (
            T_COLUMNS,
            COLUMNS_EMPTY_PAGE,
            COLUMNS_INDEX_PAGE,
            COLUMNS_DATA_PAGE,
        ),
        (T_MENU, MENU_EMPTY_PAGE, MENU_INDEX_PAGE, MENU_DATA_PAGE),
        (
            T_UNKNOWN_12,
            UNKNOWN_12_EMPTY_PAGE,
            UNKNOWN_12_INDEX_PAGE,
            UNKNOWN_12_DATA_PAGE,
        ),
        (
            T_HISTORY,
            HISTORY_EMPTY_PAGE,
            HISTORY_INDEX_PAGE,
            HISTORY_DATA_PAGE,
        ),
    ];
    write_file_header(
        &mut file,
        num_pages,
        // The hardware rejects the database when page 0 lags behind a data page.
        highest_sequence + 1,
        &pointers,
    );

    Ok(file.into_bytes())
}

// ---------------------------------------------------------------------
// Page buffer and allocation
// ---------------------------------------------------------------------

/// The raw image being assembled, one `PAGE_SIZE`-sized page at a time.
struct FileBuf {
    data: Vec<u8>,
}

impl FileBuf {
    fn new(num_pages: usize) -> Self {
        Self {
            data: vec![0u8; num_pages * PAGE_SIZE],
        }
    }

    fn page_mut(&mut self, index: u32) -> &mut [u8] {
        let start = index as usize * PAGE_SIZE;
        &mut self.data[start..start + PAGE_SIZE]
    }

    fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}

/// Where one table's pages live.
struct Allocation {
    /// Whether the table's rows fit on the page it starts on.
    data_pages: Vec<u32>,
    /// Page the table grows into next.
    empty: u32,
    /// Last page of the table's chain.
    last: u32,
}

impl Allocation {
    fn first(&self) -> u32 {
        self.data_pages[0]
    }
}

/// Assign data pages to a table: the fixed `first` page plus overflow pages
/// drawn from the monotonic `next` counter. A table that fits on one page keeps
/// its reserved empty-candidate; a table that overflows gets a fresh trailing
/// empty page so no two tables collide.
fn assign(
    table: &'static str,
    chunks: &[Range<usize>],
    first: u32,
    reserved_empty: u32,
    next: &mut u32,
) -> Allocation {
    debug_assert!(!chunks.is_empty(), "{table} must always write a data page");
    let mut data_pages = vec![first];
    for _ in 1..chunks.len() {
        data_pages.push(*next);
        *next += 1;
    }
    let empty = if chunks.len() > 1 {
        let page = *next;
        *next += 1;
        page
    } else {
        reserved_empty
    };
    let last = *data_pages.last().unwrap_or(&first);
    Allocation {
        data_pages,
        empty,
        last,
    }
}

/// Assign the artwork table's data pages.
///
/// Artwork always starts on the fixed page after its index page and takes a
/// fresh trailing empty-candidate page whenever it has rows, which is the shape
/// the hardware-validated reference exporter writes; Rekordbox leaves the
/// pointer at `28` only for the empty table (`empty = 28`, `last = 27`).
fn assign_artwork(chunks: &[Range<usize>], next: &mut u32) -> Allocation {
    debug_assert!(!chunks.is_empty(), "artwork must always write a data page");
    let mut data_pages = vec![ARTWORK_FIRST_DATA_PAGE];
    for _ in 1..chunks.len() {
        data_pages.push(*next);
        *next += 1;
    }
    let empty = *next;
    *next += 1;
    let last = *data_pages.last().unwrap_or(&ARTWORK_FIRST_DATA_PAGE);
    Allocation {
        data_pages,
        empty,
        last,
    }
}

/// Greedily pack rows into pages by exact size, accounting for the row-group
/// footer that grows with the row count. Always yields at least one (possibly
/// empty) chunk, so every table writes a data page.
fn chunk_rows(table: &'static str, rows: &[Vec<u8>]) -> Result<Vec<Range<usize>>, PdbError> {
    if rows.is_empty() {
        // One empty chunk, so every table still writes a data page.
        return Ok(vec![Range::default()]);
    }

    // The largest row this table can hold, checked once per row below.
    let mut chunks = Vec::new();
    let mut start = 0;
    let mut heap = 0usize;
    let mut count = 0usize;
    for (index, row) in rows.iter().enumerate() {
        let aligned = align4(row.len());
        if HEAP_START + aligned + row_group_bytes(1) > PAGE_SIZE {
            return Err(PdbError::RowTooLarge {
                table,
                bytes: row.len(),
            });
        }
        let full = count == MAX_ROWS_PER_PAGE
            || (count > 0 && HEAP_START + heap + aligned + row_group_bytes(count + 1) > PAGE_SIZE);
        if full {
            chunks.push(start..index);
            start = index;
            heap = 0;
            count = 0;
        }
        heap += aligned;
        count += 1;
    }
    chunks.push(start..rows.len());
    Ok(chunks)
}

fn align4(n: usize) -> usize {
    (n + 3) & !3
}

/// Total size of the row-offset footer for `num_rows` rows: full groups hold 16
/// offsets, the last one may be partial, and each group carries two flags words.
fn row_group_bytes(num_rows: usize) -> usize {
    let full = num_rows / 16;
    let partial = num_rows % 16;
    full * 36 + if partial > 0 { partial * 2 + 4 } else { 0 }
}

// ---------------------------------------------------------------------
// Page headers
// ---------------------------------------------------------------------

/// The fixed 0x28-byte page header.
#[derive(Clone, Copy)]
struct PageHeader {
    table_type: u32,
    page_index: u32,
    next_page: u32,
    /// Rows stored on this page (13 packed bits).
    num_rows: u16,
    /// Rows the row groups mark as present (11 packed bits).
    num_rows_valid: u16,
    /// Cross-page row-sequence counter.
    sequence: u32,
    /// `0x24` for a data page, `0x64` for an index page.
    page_flags: u8,
    /// `0x0001` on row pages, the row count on the fixed schema pages,
    /// `0x1FFF` on index pages.
    unknown5: u16,
    /// Row count minus one, `0` on the fixed schema pages, `0x1FFF` on index
    /// pages.
    num_rows_large: u16,
    /// Index pages: how many entries have been written. Zero on data pages.
    next_index_entry: u16,
}

impl PageHeader {
    /// A data page holding `num_rows` rows, all of them present.
    fn data(
        table_type: u32,
        page_index: u32,
        next_page: u32,
        num_rows: usize,
        sequence: u32,
        unknown5: u16,
        num_rows_large: u16,
    ) -> Self {
        Self {
            table_type,
            page_index,
            next_page,
            num_rows: num_rows as u16,
            num_rows_valid: num_rows as u16,
            sequence,
            page_flags: 0x24,
            unknown5,
            num_rows_large,
            next_index_entry: 0,
        }
    }

    /// An index ("header") page: no rows, only pointers.
    fn index(table_type: u32, page_index: u32, next_page: u32, entries: usize) -> Self {
        Self {
            table_type,
            page_index,
            next_page,
            num_rows: 0,
            num_rows_valid: 0,
            // Index pages all carry sequence 1, as Rekordbox writes them.
            sequence: 1,
            page_flags: 0x64,
            unknown5: 0x1FFF,
            num_rows_large: 0x1FFF,
            next_index_entry: entries as u16,
        }
    }
}

/// Serialize the packed 13-bit row count and 11-bit valid row count into the
/// three bytes at `0x18..0x1B`.
fn packed_row_counts(rows: u32, valid: u32) -> [u8; 3] {
    [
        (rows & 0xFF) as u8,
        (((rows >> 8) & 0x1F) as u8) | (((valid & 0x07) as u8) << 5),
        (valid >> 3) as u8,
    ]
}

fn write_page_header(page: &mut [u8], header: &PageHeader, free_size: u16, used_size: u16) {
    let packed = packed_row_counts(u32::from(header.num_rows), u32::from(header.num_rows_valid));
    page[0x04..0x08].copy_from_slice(&header.page_index.to_le_bytes());
    page[0x08..0x0C].copy_from_slice(&header.table_type.to_le_bytes());
    page[0x0C..0x10].copy_from_slice(&header.next_page.to_le_bytes());
    page[0x10..0x14].copy_from_slice(&header.sequence.to_le_bytes());
    page[0x18..0x1B].copy_from_slice(&packed);
    page[0x1B] = header.page_flags;
    page[0x1C..0x1E].copy_from_slice(&free_size.to_le_bytes());
    page[0x1E..0x20].copy_from_slice(&used_size.to_le_bytes());
    page[0x20..0x22].copy_from_slice(&header.unknown5.to_le_bytes());
    page[0x22..0x24].copy_from_slice(&header.num_rows_large.to_le_bytes());
    if header.page_flags & 0x40 != 0 {
        // Index pages carry the index header's magic and entry cursor here.
        page[0x24..0x26].copy_from_slice(&0x03ECu16.to_le_bytes());
        page[0x26..0x28].copy_from_slice(&header.next_index_entry.to_le_bytes());
    }
}

/// Write a table's index page: the page index, the page its rows start on, the
/// fixed magic words and, for the Tracks and History tables, one entry per data
/// page (the hardware walks this list).
fn write_index_page(
    file: &mut FileBuf,
    page_index: u32,
    table_type: u32,
    next_page: u32,
    indexed_pages: &[u32],
) {
    let header = PageHeader::index(table_type, page_index, next_page, indexed_pages.len());
    let page = file.page_mut(page_index);
    write_page_header(page, &header, 0, 0);

    // The page the table's rows start on. A table with no rows at all points at
    // the page after its index page, which is what Rekordbox writes for the
    // tables Mixar has nothing to put in.
    let first_data = indexed_pages.first().copied().unwrap_or(next_page);
    let mut w = HEAP_START;
    let mut put = |value: u32| {
        page[w..w + 4].copy_from_slice(&value.to_le_bytes());
        w += 4;
    };
    put(page_index);
    put(first_data);
    put(NO_PAGE);
    put(0);
    put(0x1FFF_0000 | indexed_pages.len() as u32);
    for data_page in indexed_pages {
        put(data_page << 3);
    }
    // Every remaining index entry is the "empty slot" marker, and the last 20
    // bytes of an index page stay zero.
    while w < PAGE_SIZE - 20 {
        page[w..w + 4].copy_from_slice(&0x1FFF_FFF8u32.to_le_bytes());
        w += 4;
    }
}

// ---------------------------------------------------------------------
// Data pages
// ---------------------------------------------------------------------

/// How a page's row-group footer stores its two flag words. The reference
/// exporter matches Rekordbox per table; all three layouts coincide for a full
/// group.
#[derive(Clone, Copy)]
enum RowGroupLayout {
    /// `(present bits, highest present bit)`.
    PresenceThenHighBit,
    /// `(present bits, present bits)`.
    PresenceThenIdentity,
    /// `(highest present bit, present bits)`.
    HighBitThenPresence,
}

/// Lay rows into a page heap, write the page header, and append the
/// reverse-ordered row-group footer.
fn build_data_page(
    table: &'static str,
    header: &mut PageHeader,
    rows: &[Vec<u8>],
    layout: RowGroupLayout,
    page: &mut [u8],
) -> Result<(), PdbError> {
    let mut heap = Vec::new();
    let mut offsets = Vec::with_capacity(rows.len());
    for row in rows {
        offsets.push(heap.len() as u16);
        heap.extend_from_slice(row);
        while heap.len() % 4 != 0 {
            heap.push(0);
        }
    }
    let footer = row_group_bytes(rows.len());
    if HEAP_START + heap.len() + footer > PAGE_SIZE {
        return Err(PdbError::RowTooLarge {
            table,
            bytes: heap.len(),
        });
    }

    let free_size = (PAGE_SIZE - HEAP_START - heap.len() - footer) as u16;
    write_page_header(page, header, free_size, heap.len() as u16);
    page[HEAP_START..HEAP_START + heap.len()].copy_from_slice(&heap);
    write_row_groups(page, rows.len(), &offsets, layout);
    Ok(())
}

fn write_row_groups(page: &mut [u8], num_rows: usize, offsets: &[u16], layout: RowGroupLayout) {
    let footer = row_group_bytes(num_rows);
    let mut pos = PAGE_SIZE - footer;
    for group in (0..num_rows.div_ceil(16)).rev() {
        let start = group * 16;
        let end = (start + 16).min(num_rows);
        let count = end - start;
        let present = if count == 16 {
            u16::MAX
        } else {
            (1u16 << count) - 1
        };
        // Offsets are stored with the last row of the group first.
        for slot in (start..end).rev() {
            page[pos..pos + 2].copy_from_slice(&offsets[slot].to_le_bytes());
            pos += 2;
        }
        let (first, second) = match layout {
            RowGroupLayout::PresenceThenHighBit => (present, highest_bit(present)),
            RowGroupLayout::PresenceThenIdentity => (present, present),
            RowGroupLayout::HighBitThenPresence => {
                let high = if present == u16::MAX {
                    present
                } else {
                    highest_bit(present)
                };
                (high, present)
            }
        };
        page[pos..pos + 2].copy_from_slice(&first.to_le_bytes());
        pos += 2;
        page[pos..pos + 2].copy_from_slice(&second.to_le_bytes());
        pos += 2;
    }
}

/// The highest set bit of `flags`, or zero when none is set.
fn highest_bit(flags: u16) -> u16 {
    if flags == 0 {
        0
    } else {
        1u16 << (15 - flags.leading_zeros())
    }
}

// ---------------------------------------------------------------------
// Tables whose rows come from the input
// ---------------------------------------------------------------------

struct TableCfg {
    /// Sequence of the table's first row.
    base_seq: u32,
    /// Sequence added per row.
    seq_step: u32,
    /// Rekordbox bumps the sequence by one for an eleven-row final page.
    plus_one_at_11: bool,
    /// Rekordbox numbers rows within the page (`0x20 * row index`).
    index_shift: bool,
}

/// Write all data pages of a table whose pages hold rows of one uniform shape,
/// and return the last sequence it used.
fn write_rows_table(
    file: &mut FileBuf,
    table: &'static str,
    table_type: u32,
    alloc: &Allocation,
    chunks: &[Range<usize>],
    rows: &mut [Vec<u8>],
    cfg: TableCfg,
) -> Result<u32, PdbError> {
    let mut cumulative = cfg.base_seq;
    for (chunk_index, range) in chunks.iter().enumerate() {
        let n = range.len();
        let is_last = chunk_index + 1 == chunks.len();
        let page_num = alloc.data_pages[chunk_index];
        let next_page = if is_last {
            alloc.empty
        } else {
            alloc.data_pages[chunk_index + 1]
        };

        let base = if chunk_index == 0 {
            cumulative + (n.saturating_sub(1) as u32) * cfg.seq_step
        } else {
            cumulative + (n as u32) * cfg.seq_step
        };
        let sequence = if cfg.plus_one_at_11 && n >= 11 && is_last {
            base + 1
        } else {
            base
        };
        cumulative = sequence;

        if cfg.index_shift {
            for (local, row_index) in range.clone().enumerate() {
                let shift = (local as u16).wrapping_mul(0x20);
                rows[row_index][2..4].copy_from_slice(&shift.to_le_bytes());
            }
        }

        let mut header = PageHeader::data(
            table_type,
            page_num,
            next_page,
            n,
            sequence,
            0x0001,
            n.saturating_sub(1) as u16,
        );
        build_data_page(
            table,
            &mut header,
            &rows[range.clone()],
            RowGroupLayout::PresenceThenHighBit,
            file.page_mut(page_num),
        )?;
    }
    Ok(cumulative)
}

/// The fixed `keys` table: all 24 keys, so any track's key id resolves.
fn write_keys_page(file: &mut FileBuf, track_count: usize) -> Result<u32, PdbError> {
    // Rekordbox stamps a different sequence once a database holds tracks.
    let sequence = if track_count <= 1 { 0x0A } else { 0x1B };
    let rows: Vec<Vec<u8>> = KEY_TABLE
        .iter()
        .map(|(id, name)| {
            let mut row = Vec::new();
            row.extend_from_slice(&id.to_le_bytes());
            row.extend_from_slice(&id.to_le_bytes());
            row.extend_from_slice(&encode(name));
            row
        })
        .collect();
    let n = rows.len();
    let mut header = PageHeader::data(
        T_KEYS,
        KEYS_DATA_PAGE,
        KEYS_EMPTY_PAGE,
        n,
        sequence,
        0x0001,
        (n - 1) as u16,
    );
    // The fixed 24 rows always fit on one page.
    build_data_page(
        "keys",
        &mut header,
        &rows,
        RowGroupLayout::PresenceThenHighBit,
        file.page_mut(KEYS_DATA_PAGE),
    )?;
    Ok(sequence)
}

/// The fixed `colors` table: the eight color labels a track can carry.
fn write_colors_page(file: &mut FileBuf) -> Result<u32, PdbError> {
    let colors = [
        (1u8, "Pink"),
        (2, "Red"),
        (3, "Orange"),
        (4, "Yellow"),
        (5, "Green"),
        (6, "Aqua"),
        (7, "Blue"),
        (8, "Purple"),
    ];
    let rows: Vec<Vec<u8>> = colors
        .iter()
        .map(|(index, name)| {
            let mut row = Vec::new();
            row.extend_from_slice(&0u32.to_le_bytes());
            row.push(*index);
            row.push(*index);
            row.extend_from_slice(&0u16.to_le_bytes());
            row.extend_from_slice(&encode(name));
            row
        })
        .collect();
    let mut header = PageHeader::data(
        T_COLORS,
        COLORS_DATA_PAGE,
        COLORS_EMPTY_PAGE,
        rows.len(),
        2,
        rows.len() as u16,
        0,
    );
    build_data_page(
        "colors",
        &mut header,
        &rows,
        RowGroupLayout::PresenceThenIdentity,
        file.page_mut(COLORS_DATA_PAGE),
    )?;
    Ok(2)
}

/// Rekordbox's fixed `columns` schema: the browse categories, in the order the
/// hardware expects, as `(id, kind, label)`.
const COLUMNS: [(u16, u16, &str); 27] = [
    (0x01, 0x80, "GENRE"),
    (0x02, 0x81, "ARTIST"),
    (0x03, 0x82, "ALBUM"),
    (0x04, 0x83, "TRACK"),
    (0x05, 0x85, "BPM"),
    (0x06, 0x86, "RATING"),
    (0x07, 0x87, "YEAR"),
    (0x08, 0x88, "REMIXER"),
    (0x09, 0x89, "LABEL"),
    (0x0A, 0x8A, "ORIGINAL ARTIST"),
    (0x0B, 0x8B, "KEY"),
    (0x0C, 0x8D, "CUE"),
    (0x0D, 0x8E, "COLOR"),
    (0x0E, 0x92, "TIME"),
    (0x0F, 0x93, "BITRATE"),
    (0x10, 0x94, "FILE NAME"),
    (0x11, 0x84, "PLAYLIST"),
    (0x12, 0x98, "HOT CUE BANK"),
    (0x13, 0x95, "HISTORY"),
    (0x14, 0x91, "SEARCH"),
    (0x15, 0x96, "COMMENTS"),
    (0x16, 0x8C, "DATE ADDED"),
    (0x17, 0x97, "DJ PLAY COUNT"),
    (0x18, 0x90, "FOLDER"),
    (0x19, 0xA1, "DEFAULT"),
    (0x1A, 0xA2, "ALPHABET"),
    (0x1B, 0xAA, "MATCHING"),
];

/// The `columns` table. Its page header differs from every other table: the
/// valid-row-count slot holds the row count, `num_rows_large` is zero and the
/// sequence is fixed.
fn write_columns_page(file: &mut FileBuf) -> Result<u32, PdbError> {
    let rows: Vec<Vec<u8>> = COLUMNS
        .iter()
        .map(|&(id, kind, label)| {
            let mut row = Vec::with_capacity(24);
            row.extend_from_slice(&id.to_le_bytes());
            row.extend_from_slice(&kind.to_le_bytes());
            row.extend_from_slice(&encode(&format!("\u{FFFA}{label}\u{FFFB}")));
            row
        })
        .collect();
    let mut header = PageHeader::data(
        T_COLUMNS,
        COLUMNS_DATA_PAGE,
        COLUMNS_EMPTY_PAGE,
        rows.len(),
        3,
        rows.len() as u16,
        0,
    );
    build_data_page(
        "columns",
        &mut header,
        &rows,
        RowGroupLayout::PresenceThenIdentity,
        file.page_mut(COLUMNS_DATA_PAGE),
    )?;
    Ok(3)
}

/// The fixed browse-menu rows firmware expects, as
/// `(category_id, content_pointer, kind, visibility, sort_order)`.
const MENU_ROWS: [(u16, u16, u8, u8, u16); 22] = [
    (0x01, 0x01, 0x63, 0x01, 0x00),
    (0x05, 0x06, 0x05, 0x01, 0x00),
    (0x06, 0x07, 0x63, 0x01, 0x00),
    (0x07, 0x08, 0x63, 0x01, 0x00),
    (0x08, 0x09, 0x63, 0x01, 0x00),
    (0x09, 0x0A, 0x63, 0x01, 0x00),
    (0x0A, 0x0B, 0x63, 0x01, 0x00),
    (0x0D, 0x0F, 0x63, 0x01, 0x00),
    (0x0E, 0x13, 0x04, 0x01, 0x00),
    (0x0F, 0x14, 0x06, 0x01, 0x00),
    (0x10, 0x15, 0x63, 0x01, 0x00),
    (0x12, 0x17, 0x63, 0x01, 0x00),
    (0x02, 0x02, 0x02, 0x00, 0x01),
    (0x03, 0x03, 0x03, 0x00, 0x02),
    (0x04, 0x04, 0x01, 0x00, 0x03),
    (0x0B, 0x0C, 0x63, 0x00, 0x04),
    (0x11, 0x05, 0x63, 0x00, 0x05),
    (0x13, 0x16, 0x63, 0x00, 0x06),
    (0x14, 0x12, 0x63, 0x00, 0x07),
    (0x1B, 0x1A, 0x63, 0x02, 0x08),
    (0x18, 0x11, 0x63, 0x00, 0x09),
    (0x16, 0x1B, 0x63, 0x00, 0x0A),
];

/// The `menu` table: which browse categories the player offers, and in which
/// order. Content-independent, so its rows are a constant.
fn write_menu_page(file: &mut FileBuf) -> Result<u32, PdbError> {
    let rows: Vec<Vec<u8>> = MENU_ROWS
        .iter()
        .map(|&(category, content, kind, visibility, sort_order)| {
            let mut row = Vec::with_capacity(8);
            row.extend_from_slice(&category.to_le_bytes());
            row.extend_from_slice(&content.to_le_bytes());
            row.push(kind);
            row.push(visibility);
            row.extend_from_slice(&sort_order.to_le_bytes());
            row
        })
        .collect();
    let mut header = PageHeader::data(
        T_MENU,
        MENU_DATA_PAGE,
        MENU_EMPTY_PAGE,
        rows.len(),
        4,
        rows.len() as u16,
        0,
    );
    build_data_page(
        "menu",
        &mut header,
        &rows,
        RowGroupLayout::PresenceThenIdentity,
        file.page_mut(MENU_DATA_PAGE),
    )?;
    Ok(4)
}

/// Table `0x12`, whose meaning is not established; the rows are the constant
/// set firmware expects, as `(field, field, field)`.
const UNKNOWN_12_ROWS: [(u16, u16, u32); 17] = [
    (0x01, 0x06, 0x0001),
    (0x15, 0x07, 0x0001),
    (0x0E, 0x08, 0x0001),
    (0x08, 0x09, 0x0001),
    (0x09, 0x0A, 0x0001),
    (0x0A, 0x0B, 0x0001),
    (0x0F, 0x0D, 0x0001),
    (0x0D, 0x0F, 0x0001),
    (0x17, 0x10, 0x0001),
    (0x16, 0x11, 0x0001),
    (0x19, 0x00, 0x0100),
    (0x1A, 0x01, 0x0200),
    (0x02, 0x02, 0x0300),
    (0x03, 0x03, 0x0400),
    (0x05, 0x04, 0x0500),
    (0x06, 0x05, 0x0600),
    (0x0B, 0x0C, 0x0700),
];

/// The fixed table `0x12`. Its row-group footer differs from the other fixed
/// tables, matching what Rekordbox writes.
fn write_unknown_12_page(file: &mut FileBuf) -> Result<u32, PdbError> {
    let rows: Vec<Vec<u8>> = UNKNOWN_12_ROWS
        .iter()
        .map(|&(first, second, third)| {
            let mut row = Vec::with_capacity(8);
            row.extend_from_slice(&first.to_le_bytes());
            row.extend_from_slice(&second.to_le_bytes());
            row.extend_from_slice(&third.to_le_bytes());
            row
        })
        .collect();
    let mut header = PageHeader::data(
        T_UNKNOWN_12,
        UNKNOWN_12_DATA_PAGE,
        UNKNOWN_12_EMPTY_PAGE,
        rows.len(),
        5,
        rows.len() as u16,
        0,
    );
    build_data_page(
        "table 0x12",
        &mut header,
        &rows,
        RowGroupLayout::HighBitThenPresence,
        file.page_mut(UNKNOWN_12_DATA_PAGE),
    )?;
    Ok(5)
}

/// The `history` table: the sync log. One row describes this export; the
/// hardware rejects a database whose history table has no rows.
fn write_history_page(
    file: &mut FileBuf,
    track_count: usize,
    export_date: &str,
) -> Result<u32, PdbError> {
    let mut row = Vec::with_capacity(40);
    row.extend_from_slice(&0x0280u16.to_le_bytes());
    row.extend_from_slice(&0u16.to_le_bytes());
    row.extend_from_slice(&(track_count.min(u32::MAX as usize) as u32).to_le_bytes());
    row.extend_from_slice(&0u32.to_le_bytes());
    row.extend_from_slice(&encode(export_date));
    row.extend_from_slice(&0x1E19u16.to_le_bytes());
    row.extend_from_slice(&encode("1000"));
    row.extend_from_slice(&encode(""));
    while row.len() < 40 {
        row.push(0);
    }

    let mut header = PageHeader::data(
        T_HISTORY,
        HISTORY_DATA_PAGE,
        HISTORY_EMPTY_PAGE,
        1,
        1,
        0x0001,
        0,
    );
    build_data_page(
        "history",
        &mut header,
        &[row],
        RowGroupLayout::HighBitThenPresence,
        file.page_mut(HISTORY_DATA_PAGE),
    )?;
    Ok(1)
}

// ---------------------------------------------------------------------
// Row serializers
// ---------------------------------------------------------------------

/// The distinct artists, albums and genres a track slice references, in
/// first-appearance order. Interning in track order (rather than by hash-map
/// iteration) is what makes `build` deterministic.
struct Entities {
    artists: Vec<String>,
    albums: Vec<String>,
    genres: Vec<String>,
    artist_id: HashMap<String, u32>,
    album_id: HashMap<String, u32>,
    genre_id: HashMap<String, u32>,
}

impl Entities {
    fn build(tracks: &[CdjTrack]) -> Self {
        let mut entities = Self {
            artists: Vec::new(),
            albums: Vec::new(),
            genres: Vec::new(),
            artist_id: HashMap::new(),
            album_id: HashMap::new(),
            genre_id: HashMap::new(),
        };
        for track in tracks {
            intern(
                &mut entities.artists,
                &mut entities.artist_id,
                &track.artist,
            );
            intern(&mut entities.albums, &mut entities.album_id, &track.album);
            if let Some(genre) = &track.genre {
                if !genre.is_empty() {
                    intern(&mut entities.genres, &mut entities.genre_id, genre);
                }
            }
        }
        entities
    }
}

fn intern(list: &mut Vec<String>, ids: &mut HashMap<String, u32>, value: &str) {
    if !ids.contains_key(value) {
        let id = (list.len() + 1) as u32;
        list.push(value.to_owned());
        ids.insert(value.to_owned(), id);
    }
}

/// One `tracks` row: the fixed 0x5E-byte header, the 21 string offsets and the
/// inline DeviceSQL strings they point at.
fn track_row(track: &CdjTrack, entities: &Entities, index: usize, export_date: &str) -> Vec<u8> {
    let track_id = (index + 1) as u32;
    let artist_id = entities.artist_id.get(&track.artist).copied().unwrap_or(0);
    let album_id = entities.album_id.get(&track.album).copied().unwrap_or(0);
    let genre_id = track
        .genre
        .as_ref()
        .and_then(|genre| entities.genre_id.get(genre))
        .copied()
        .unwrap_or(0);
    // Zero means "no key"; anything outside the 24-row table is unknown too.
    let key_id = track
        .key_id
        .filter(|key| (1..=24).contains(key))
        .unwrap_or(0);
    // A zero sample rate or bit depth would make the row unplayable metadata.
    let sample_rate = if track.sample_rate_hz == 0 {
        44_100
    } else {
        track.sample_rate_hz
    };
    let bit_depth = if track.bit_depth == 0 {
        16
    } else {
        track.bit_depth
    };
    let tempo = match track.bpm {
        Some(bpm) if bpm.is_finite() && bpm > 0.0 => {
            let centi = (bpm * 100.0).round();
            if centi > f64::from(u32::MAX) {
                0
            } else {
                centi as u32
            }
        }
        _ => 0,
    };
    let filename = track
        .device_audio_path
        .rsplit('/')
        .next()
        .unwrap_or(&track.device_audio_path);
    let date_added = track.date_added.as_deref().unwrap_or("");

    let mut row = Vec::with_capacity(0x88);
    let u16_ = |row: &mut Vec<u8>, value: u16| row.extend_from_slice(&value.to_le_bytes());
    let u32_ = |row: &mut Vec<u8>, value: u32| row.extend_from_slice(&value.to_le_bytes());

    u16_(&mut row, 0x0024); // subtype: 16-bit string offsets
    u16_(&mut row, 0); // index_shift, patched per page
    u32_(&mut row, 0x000C_0700); // bitmask
    u32_(&mut row, sample_rate);
    u32_(&mut row, 0); // composer_id
    u32_(&mut row, track.file_size.min(u64::from(u32::MAX)) as u32);
    u32_(&mut row, (track_id + 5) | 0x100); // waveform-ready marker
    u16_(&mut row, 0xE5B6);
    u16_(&mut row, 0x6A76);
    u32_(&mut row, track.artwork_id); // artwork_id (0 = none)
    u32_(&mut row, key_id);
    u32_(&mut row, 0); // original_artist_id
    u32_(&mut row, 0); // label_id
    u32_(&mut row, 0); // remixer_id
    u32_(&mut row, track.bitrate_kbps.unwrap_or(0));
    u32_(&mut row, 0); // track_number
    u32_(&mut row, tempo);
    u32_(&mut row, genre_id);
    u32_(&mut row, album_id);
    u32_(&mut row, artist_id);
    u32_(&mut row, track_id);
    u16_(&mut row, 0); // disc_number
    u16_(&mut row, 0); // play_count
    u16_(&mut row, 0); // year
    u16_(&mut row, bit_depth);
    u16_(
        &mut row,
        track.duration_secs.min(u32::from(u16::MAX)) as u16,
    );
    u16_(&mut row, 0x0029);
    row.push(0); // color_id
    row.push(0); // rating
    u16_(&mut row, track.file_type as u16);
    u16_(&mut row, 0x0003); // string-offset table marker
    debug_assert_eq!(row.len(), 0x5E);

    // 21 string offsets, then the DeviceSQL strings they point at.
    const STRING_DATA_START: usize = 0x5E + 21 * 2;
    let mut string_data = Vec::new();
    let mut offsets = [0u16; 21];
    let mut add = |index: usize, bytes: &[u8]| {
        offsets[index] = (STRING_DATA_START + string_data.len()) as u16;
        string_data.extend_from_slice(bytes);
    };
    add(0, &[0x03]); // isrc
    add(1, &[0x03]); // lyricist
    add(2, &encode("3"));
    add(3, &encode("2"));
    add(4, &[0x03]);
    add(5, &[0x03]); // message
    add(6, &[0x03]); // publish track information
    add(7, &encode("ON")); // autoload_hotcues
    add(8, &[0x03]);
    add(9, &[0x03]);
    add(10, &encode(date_added));
    add(11, &[0x03]); // release_date
    add(12, &[0x03]); // mix_name
    add(13, &[0x03]);
    add(14, &encode(&track.analyze_path));
    add(15, &encode(export_date)); // analyze_date
    add(16, &[0x03]); // comment
    add(17, &encode(&track.title));
    add(18, &[0x03]);
    add(19, &encode(filename));
    add(20, &encode(&track.device_audio_path));

    for offset in offsets {
        row.extend_from_slice(&offset.to_le_bytes());
    }
    debug_assert_eq!(row.len(), STRING_DATA_START);
    row.extend_from_slice(&string_data);
    row
}

/// One `artists` row: subtype 0x60 selects one-byte name offsets.
fn artist_row(name: &str, id: usize) -> Vec<u8> {
    let mut row = Vec::with_capacity(28);
    row.extend_from_slice(&0x0060u16.to_le_bytes());
    row.extend_from_slice(&0u16.to_le_bytes()); // index_shift, patched per page
    row.extend_from_slice(&(id as u32).to_le_bytes());
    row.push(0x03); // one-byte offset table marker
    row.push(10); // name offset
    row.extend_from_slice(&encode(name));
    while row.len() < 28 {
        row.push(0);
    }
    row
}

/// One `albums` row.
fn album_row(name: &str, id: usize) -> Vec<u8> {
    let mut row = Vec::with_capacity(40);
    row.extend_from_slice(&0x0080u16.to_le_bytes());
    row.extend_from_slice(&0u16.to_le_bytes()); // index_shift, patched per page
    row.extend_from_slice(&0u32.to_le_bytes());
    row.extend_from_slice(&0u32.to_le_bytes()); // artist_id
    row.extend_from_slice(&(id as u32).to_le_bytes());
    row.extend_from_slice(&0u32.to_le_bytes());
    row.push(0x03); // one-byte offset table marker
    row.push(22); // name offset
    row.extend_from_slice(&encode(name));
    while row.len() < 40 {
        row.push(0);
    }
    row
}

/// One `genres` row.
fn genre_row(name: &str, id: usize) -> Vec<u8> {
    let mut row = Vec::new();
    row.extend_from_slice(&(id as u32).to_le_bytes());
    row.extend_from_slice(&encode(name));
    row
}

/// Stride the reference exporter packs artwork rows at; the DeviceSQL path is
/// inlined in the row rather than offset-addressed. Longer paths produce longer,
/// 4-byte-aligned rows, which the page footer still addresses correctly.
const ARTWORK_ROW_SIZE: usize = 36;

/// One artwork row: the id tracks reference, then the on-drive JPEG path.
fn artwork_row(artwork: &CdjArtwork) -> Vec<u8> {
    let mut row = Vec::with_capacity(ARTWORK_ROW_SIZE);
    row.extend_from_slice(&artwork.id.to_le_bytes());
    row.extend_from_slice(&encode(&artwork.path));
    while row.len() < ARTWORK_ROW_SIZE {
        row.push(0);
    }
    row
}

/// One `playlist tree` row: a playlist at the root, with the id playlist
/// entries reference.
fn playlist_tree_row(name: &str, index: usize) -> Vec<u8> {
    let mut row = Vec::new();
    row.extend_from_slice(&0u32.to_le_bytes()); // parent_id (root)
    row.extend_from_slice(&0u32.to_le_bytes());
    row.extend_from_slice(&(index as u32).to_le_bytes()); // sort_order
    row.extend_from_slice(&((index + 1) as u32).to_le_bytes()); // playlist id
    row.extend_from_slice(&0u32.to_le_bytes()); // node_is_folder
    row.extend_from_slice(&encode(name));
    row
}

/// One `playlist entries` row per playlist entry, in playlist order.
fn playlist_entry_rows(playlists: &[CdjPlaylist]) -> Vec<Vec<u8>> {
    let mut rows = Vec::new();
    for (playlist_index, playlist) in playlists.iter().enumerate() {
        let playlist_id = (playlist_index + 1) as u32;
        for (position, &track_index) in playlist.entries.iter().enumerate() {
            let mut row = Vec::with_capacity(12);
            row.extend_from_slice(&((position + 1) as u32).to_le_bytes());
            row.extend_from_slice(&((track_index + 1) as u32).to_le_bytes());
            row.extend_from_slice(&playlist_id.to_le_bytes());
            rows.push(row);
        }
    }
    rows
}

// ---------------------------------------------------------------------
// Page 0
// ---------------------------------------------------------------------

/// Write the file header and the 20 table pointers, as
/// `(table type, empty-candidate page, first page, last page)`.
fn write_file_header(
    file: &mut FileBuf,
    num_pages: usize,
    sequence: u32,
    pointers: &[(u32, u32, u32, u32); 20],
) {
    let next_unused = (num_pages as u32).max(OVERFLOW_START);
    let page = file.page_mut(0);
    page[0x00..0x04].copy_from_slice(&[0u8; 4]); // magic
    page[0x04..0x08].copy_from_slice(&(PAGE_SIZE as u32).to_le_bytes());
    page[0x08..0x0C].copy_from_slice(&NUM_TABLES.to_le_bytes());
    page[0x0C..0x10].copy_from_slice(&next_unused.to_le_bytes());
    page[0x10..0x14].copy_from_slice(&5u32.to_le_bytes());
    page[0x14..0x18].copy_from_slice(&sequence.to_le_bytes());
    page[0x18..0x1C].copy_from_slice(&[0u8; 4]); // padding

    let mut w = 0x1C;
    for &(table_type, empty, first, last) in pointers {
        page[w..w + 4].copy_from_slice(&table_type.to_le_bytes());
        page[w + 4..w + 8].copy_from_slice(&empty.to_le_bytes());
        page[w + 8..w + 12].copy_from_slice(&first.to_le_bytes());
        page[w + 12..w + 16].copy_from_slice(&last.to_le_bytes());
        w += 16;
    }
}

#[cfg(test)]
mod tests {
    use super::super::model::CdjFileType;
    use super::*;
    use rekordcrate::pdb::io::Database;
    use rekordcrate::pdb::{
        Album, Artist, Artwork, ColumnEntry, DatabaseType, Genre, Key, PageContent, PageType,
        PlainPageType, PlaylistEntry, PlaylistTreeNode, RowVariant, Track,
    };
    use std::io::Cursor;

    const EXPORT_DATE: &str = "2026-01-02";

    fn track(title: &str, artist: &str, album: &str, genre: Option<&str>) -> CdjTrack {
        CdjTrack {
            title: title.to_owned(),
            artist: artist.to_owned(),
            album: album.to_owned(),
            genre: genre.map(str::to_owned),
            bpm: Some(120.0),
            key_id: Some(5),
            duration_secs: 200,
            file_size: 4_194_304,
            bitrate_kbps: Some(320),
            sample_rate_hz: 44_100,
            bit_depth: 16,
            file_type: CdjFileType::Mp3,
            artwork_id: 0,
            date_added: Some("2025-12-24".to_owned()),
            device_audio_path: format!("/Contents/{artist}/{album}/{title}.mp3"),
            analyze_path: format!("/PIONEER/USBANLZ/P000/0000000{}/ANLZ0000.DAT", title.len()),
        }
    }

    /// Read every row of one table back with `rekordcrate`.
    fn rows_of<T: RowVariant + Clone>(bytes: &[u8]) -> Vec<T> {
        let mut db = Database::open_non_persistent(Cursor::new(bytes), DatabaseType::Plain)
            .expect("open the database we just wrote");
        let (first, last) = {
            let (_, table) = db
                .get_header()
                .find_table(T::PAGE_TYPE)
                .expect("table present");
            (table.first_page, table.last_page)
        };
        let mut rows = Vec::new();
        let mut index = first;
        loop {
            let page = db.load_page(index).expect("load page");
            let next = page.header.next_page;
            if let PageContent::Data(data) = &page.content {
                for row in data.rows.values() {
                    if let Some(row) = row.as_variant::<T>() {
                        rows.push(row.clone());
                    }
                }
            }
            if index == last {
                break;
            }
            index = next;
        }
        rows
    }

    fn u16_at(bytes: &[u8], offset: usize) -> u16 {
        u16::from_le_bytes([bytes[offset], bytes[offset + 1]])
    }

    fn u32_at(bytes: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            bytes[offset],
            bytes[offset + 1],
            bytes[offset + 2],
            bytes[offset + 3],
        ])
    }

    fn small_export() -> (Vec<CdjTrack>, Vec<CdjPlaylist>) {
        let tracks = vec![
            track("One", "Artist A", "Album X", Some("House")),
            track("Two", "Artist B", "Album Y", Some("Techno")),
            track("Three", "Artist A", "Album X", Some("House")),
        ];
        let playlists = vec![
            CdjPlaylist {
                name: "Warmup".to_owned(),
                entries: vec![2, 0],
            },
            CdjPlaylist {
                name: "Peak".to_owned(),
                entries: vec![1],
            },
        ];
        (tracks, playlists)
    }

    #[test]
    fn page_layout_and_file_header_are_well_formed() {
        let (tracks, playlists) = small_export();
        let bytes = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");

        assert_eq!(bytes.len() % PAGE_SIZE, 0);
        assert_eq!(bytes.len(), STANDARD_PAGES * PAGE_SIZE);

        // File header.
        assert_eq!(u32_at(&bytes, 0x00), 0, "magic");
        assert_eq!(u32_at(&bytes, 0x04), PAGE_SIZE as u32);
        assert_eq!(u32_at(&bytes, 0x08), NUM_TABLES);
        assert_eq!(u32_at(&bytes, 0x10), 5);
        assert_eq!(u32_at(&bytes, 0x18), 0, "padding");

        // The 20 table pointers, in the order the hardware expects.
        let expected_types = [
            0x00u32, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0A, 0x0B, 0x0C, 0x0D,
            0x0E, 0x0F, 0x10, 0x11, 0x12, 0x13,
        ];
        let mut tables_seen = Vec::new();
        for (index, expected) in expected_types.iter().enumerate() {
            let pointer = 0x1C + index * 16;
            assert_eq!(u32_at(&bytes, pointer), *expected, "table {index} type");
            let first = u32_at(&bytes, pointer + 8);
            let last = u32_at(&bytes, pointer + 12);
            assert!(first <= last, "table {index} page order");
            assert!(
                (last as usize) < STANDARD_PAGES,
                "table {index} points past the file"
            );
            assert!(
                u32_at(&bytes, pointer + 4) <= 52,
                "table {index} empty candidate is outside the reserved zone"
            );
            tables_seen.push(first);
        }
        assert_eq!(
            tables_seen,
            (0..20).map(|i| 1 + 2 * i as u32).collect::<Vec<_>>()
        );

        // Page 0's sequence must lead every data page, or the hardware reports
        // "rekordbox database not found".
        let header_sequence = u32_at(&bytes, 0x14);
        let mut highest = 0;
        for page in 1..bytes.len() / PAGE_SIZE {
            let base = page * PAGE_SIZE;
            highest = highest.max(u32_at(&bytes, base + 0x10));
        }
        assert!(
            header_sequence > highest,
            "page-0 sequence {header_sequence} must exceed every page sequence ({highest})"
        );
        assert!(u32_at(&bytes, 0x0C) >= OVERFLOW_START, "next_unused_page");

        // Each table pointer names the index page this writer put it on, and a
        // table with rows chains on to its data page.
        let with_data_pages = [0, 1, 2, 3, 5, 6, 7, 8, 16, 17, 18, 19];
        for index in 0..20 {
            let pointer = 0x1C + index * 16;
            let first = u32_at(&bytes, pointer + 8);
            let last = u32_at(&bytes, pointer + 12);
            assert_eq!(first, 1 + 2 * index as u32, "table {index} index page");
            let expected_last = if with_data_pages.contains(&index) {
                first + 1
            } else {
                first
            };
            assert_eq!(last, expected_last, "table {index} last page");
        }
    }

    #[test]
    fn tracks_round_trip_through_rekordcrate() {
        let (tracks, playlists) = small_export();
        let bytes = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");

        let parsed = rows_of::<Track>(&bytes);
        assert_eq!(parsed.len(), tracks.len());
        assert_eq!(
            parsed
                .iter()
                .map(|row| row.offsets.title.clone().into_string().unwrap())
                .collect::<Vec<_>>(),
            ["One", "Two", "Three"]
        );

        let json = serde_json::to_value(&parsed[0]).expect("serialize row");
        assert_eq!(json["subtype"], 0x0024);
        assert_eq!(json["sample_rate"], 44_100);
        assert_eq!(json["file_size"], 4_194_304);
        assert_eq!(json["bitrate"], 320);
        assert_eq!(json["tempo"], 12_000, "BPM x 100");
        assert_eq!(json["duration"], 200);
        assert_eq!(json["sample_depth"], 16);
        assert_eq!(json["file_type"], "Mp3");
        assert_eq!(json["key_id"], 5);
        assert_eq!(json["artist_id"], 1);
        assert_eq!(json["album_id"], 1);
        assert_eq!(json["genre_id"], 1);
        assert_eq!(json["id"], 1);
        assert_eq!(json["offsets"]["inner"]["title"], "One");
        assert_eq!(json["offsets"]["inner"]["filename"], "One.mp3");
        assert_eq!(
            json["offsets"]["inner"]["file_path"],
            "/Contents/Artist A/Album X/One.mp3"
        );
        assert_eq!(
            json["offsets"]["inner"]["analyze_path"],
            "/PIONEER/USBANLZ/P000/00000003/ANLZ0000.DAT"
        );
        assert_eq!(json["offsets"]["inner"]["analyze_date"], EXPORT_DATE);
        assert_eq!(json["offsets"]["inner"]["date_added"], "2025-12-24");
        assert_eq!(json["offsets"]["inner"]["autoload_hotcues"], "ON");
        assert_eq!(json["offsets"]["inner"]["isrc"], "");
        assert_eq!(json["offsets"]["inner"]["comment"], "");

        // Distinct tracks keep the ids their own metadata resolves to, and the
        // second Artist A track reuses artist 1.
        let third = serde_json::to_value(&parsed[2]).expect("serialize row");
        assert_eq!(third["id"], 3);
        assert_eq!(third["artist_id"], 1);
        assert_eq!(third["album_id"], 1);
        assert_eq!(third["genre_id"], 1);

        // Index shifts number rows within their page.
        assert_eq!(json["index_shift"], 0);
        let second = serde_json::to_value(&parsed[1]).expect("serialize row");
        assert_eq!(second["index_shift"], 0x20);
    }

    #[test]
    fn artwork_rows_round_trip_and_tracks_reference_them() {
        let (mut tracks, playlists) = small_export();
        tracks[0].artwork_id = 1;
        let artworks = [
            CdjArtwork {
                id: 1,
                path: "/PIONEER/Artwork/00001/a1.jpg".to_owned(),
            },
            CdjArtwork {
                id: 2,
                path: "/PIONEER/Artwork/00002/a1.jpg".to_owned(),
            },
        ];
        let bytes = build(&tracks, &playlists, &artworks, EXPORT_DATE).expect("build pdb");

        let parsed = rows_of::<Artwork>(&bytes);
        assert_eq!(parsed.len(), 2);
        let json = serde_json::to_value(&parsed[0]).expect("serialize row");
        assert_eq!(json["id"], 1);
        assert_eq!(json["path"], "/PIONEER/Artwork/00001/a1.jpg");
        let second = serde_json::to_value(&parsed[1]).expect("serialize row");
        assert_eq!(second["id"], 2);

        // Tracks point at the shared row; tracks without a cover keep 0.
        let parsed = rows_of::<Track>(&bytes);
        let first = serde_json::to_value(&parsed[0]).expect("serialize row");
        assert_eq!(first["artwork_id"], 1);
        let other = serde_json::to_value(&parsed[1]).expect("serialize row");
        assert_eq!(other["artwork_id"], 0);

        // The table starts on the fixed page after its index page and takes a
        // fresh page as its empty candidate, like the reference exporter.
        let db = Database::open_non_persistent(Cursor::new(&bytes), DatabaseType::Plain)
            .expect("open the database we just wrote");
        let page_size = u64::from(db.get_header().page_size);
        let (_, table) = db
            .get_header()
            .find_table(PageType::Plain(PlainPageType::Artwork))
            .expect("artwork table present");
        assert_eq!(
            table.first_page.offset(db.get_header().page_size) / page_size,
            27
        );
        assert_eq!(
            table.last_page.offset(db.get_header().page_size) / page_size,
            28
        );
    }

    #[test]
    fn artwork_row_stride_matches_the_reference() {
        let row = artwork_row(&CdjArtwork {
            id: 1,
            path: "/PIONEER/Artwork/00001/a1.jpg".to_owned(),
        });
        assert_eq!(row.len(), ARTWORK_ROW_SIZE);
        assert_eq!(&row[..4], &1u32.to_le_bytes());
        let encoded = encode("/PIONEER/Artwork/00001/a1.jpg");
        assert_eq!(&encoded, &row[4..4 + encoded.len()]);
        assert!(row[4 + encoded.len()..].iter().all(|byte| *byte == 0));

        // A long path simply produces a longer row; the page footer addresses it.
        let long = artwork_row(&CdjArtwork {
            id: 9,
            path: format!("/PIONEER/Artwork/00009/{}.jpg", "x".repeat(120)),
        });
        assert!(long.len() > ARTWORK_ROW_SIZE);
    }

    #[test]
    fn artists_albums_genres_and_keys_round_trip() {
        let (tracks, playlists) = small_export();
        let bytes = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");

        let artists = rows_of::<Artist>(&bytes);
        assert_eq!(artists.len(), 2, "one row per distinct artist");
        assert_eq!(
            artists
                .iter()
                .map(|row| row.offsets.name.clone().into_string().unwrap())
                .collect::<Vec<_>>(),
            ["Artist A", "Artist B"]
        );
        assert_eq!(artists[0].id.0, 1);
        assert_eq!(artists[1].id.0, 2);

        let albums = rows_of::<Album>(&bytes);
        assert_eq!(albums.len(), 2);
        let albums = albums
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(albums[0]["id"], 1);
        assert_eq!(albums[0]["offsets"]["inner"]["name"], "Album X");
        assert_eq!(albums[1]["offsets"]["inner"]["name"], "Album Y");

        let genres = rows_of::<Genre>(&bytes);
        let genres = genres
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect::<Vec<_>>();
        assert_eq!(genres.len(), 2);
        assert_eq!(genres[0]["id"], 1);
        assert_eq!(genres[0]["name"], "House");
        assert_eq!(genres[1]["name"], "Techno");

        let keys = rows_of::<Key>(&bytes);
        assert_eq!(keys.len(), 24, "the whole key table is always written");
        let keys = keys
            .iter()
            .map(|row| serde_json::to_value(row).unwrap())
            .collect::<Vec<_>>();
        for (index, (id, name)) in KEY_TABLE.iter().enumerate() {
            assert_eq!(keys[index]["id"], *id);
            assert_eq!(keys[index]["id2"], *id);
            assert_eq!(keys[index]["name"], *name);
        }

        // The fixed browse schema is there for the hardware to read.
        let columns = rows_of::<ColumnEntry>(&bytes);
        assert_eq!(columns.len(), COLUMNS.len());
    }

    #[test]
    fn playlists_round_trip_in_order() {
        let (tracks, playlists) = small_export();
        let bytes = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");

        let nodes = rows_of::<PlaylistTreeNode>(&bytes);
        assert_eq!(nodes.len(), playlists.len());
        assert_eq!(nodes[0].id.0, 1);
        assert_eq!(nodes[0].parent_id.0, 0, "playlists sit at the root");
        assert_eq!(nodes[0].name.clone().into_string().unwrap(), "Warmup");
        assert!(!nodes[0].is_folder());
        assert_eq!(nodes[1].id.0, 2);
        assert_eq!(nodes[1].name.clone().into_string().unwrap(), "Peak");

        let entries = rows_of::<PlaylistEntry>(&bytes);
        assert_eq!(entries.len(), 3);
        let membership: Vec<(u32, u32, u32)> = entries
            .iter()
            .map(|entry| (entry.playlist_id.0, entry.entry_index, entry.track_id.0))
            .collect();
        assert_eq!(membership, [(1, 1, 3), (1, 2, 1), (2, 1, 2)]);
    }

    #[test]
    fn empty_export_is_still_a_valid_database() {
        let bytes = build(&[], &[], &[], EXPORT_DATE).expect("build pdb");

        assert_eq!(bytes.len(), STANDARD_PAGES * PAGE_SIZE);
        assert!(rows_of::<Track>(&bytes).is_empty());
        assert!(rows_of::<PlaylistTreeNode>(&bytes).is_empty());
        assert!(rows_of::<PlaylistEntry>(&bytes).is_empty());
        assert_eq!(rows_of::<Key>(&bytes).len(), 24);
        assert_eq!(rows_of::<ColumnEntry>(&bytes).len(), COLUMNS.len());

        // The history page must carry a row: an empty history table is one of
        // the things that makes CDJ-class hardware reject the database.
        let history_page = &bytes[HISTORY_DATA_PAGE as usize * PAGE_SIZE..][..PAGE_SIZE];
        assert_eq!(u16_at(history_page, 0x20), 1, "unknown5: one row");
        assert_eq!(
            u16_at(history_page, 0x1E),
            0x28,
            "used size: one 40-byte row"
        );
        assert_eq!(u16_at(history_page, HEAP_START), 0x0280);
        assert_eq!(u32_at(history_page, HEAP_START + 4), 0, "no tracks");
        assert_eq!(
            &history_page[HEAP_START + 12..HEAP_START + 23],
            &encode(EXPORT_DATE)[..]
        );
        assert_eq!(u16_at(history_page, HEAP_START + 23), 0x1E19);
        assert_eq!(
            &history_page[HEAP_START + 25..HEAP_START + 30],
            &encode("1000")[..]
        );
    }

    #[test]
    fn tracks_and_history_index_pages_list_their_data_pages() {
        let (tracks, playlists) = small_export();
        let bytes = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");

        // Index page header: 0x1FFF/0x1FFF, the 0x03EC magic, the entry cursor,
        // this page's index, the first data page and the 0x03FFFFFF magic.
        for (index_page, data_page) in [
            (TRACKS_INDEX_PAGE, TRACKS_FIRST_DATA_PAGE),
            (HISTORY_INDEX_PAGE, HISTORY_DATA_PAGE),
        ] {
            let page = &bytes[index_page as usize * PAGE_SIZE..][..PAGE_SIZE];
            assert_eq!(u16_at(page, 0x20), 0x1FFF);
            assert_eq!(u16_at(page, 0x22), 0x1FFF);
            assert_eq!(u16_at(page, 0x24), 0x03EC);
            assert_eq!(u16_at(page, 0x26), 1, "one index entry written");
            assert_eq!(u32_at(page, 0x28), index_page);
            assert_eq!(u32_at(page, 0x2C), data_page);
            assert_eq!(u32_at(page, 0x30), NO_PAGE);
            assert_eq!(u32_at(page, 0x38), 0x1FFF_0001, "num_entries | first_empty");
            assert_eq!(u32_at(page, 0x3C), data_page << 3, "the data page's entry");
            assert_eq!(u32_at(page, PAGE_SIZE - 20), 0, "index pages end zeroed");
        }

        // Every other index page stays empty, as Rekordbox writes it.
        for index_page in [
            GENRES_INDEX_PAGE,
            ARTISTS_INDEX_PAGE,
            ALBUMS_INDEX_PAGE,
            KEYS_INDEX_PAGE,
            COLORS_INDEX_PAGE,
            COLUMNS_INDEX_PAGE,
            MENU_INDEX_PAGE,
            UNKNOWN_12_INDEX_PAGE,
        ] {
            let page = &bytes[index_page as usize * PAGE_SIZE..][..PAGE_SIZE];
            assert_eq!(u16_at(page, 0x26), 0, "page {index_page} entry cursor");
            assert_eq!(u32_at(page, 0x38), 0x1FFF_0000, "page {index_page} entries");
            assert_eq!(
                u32_at(page, 0x3C),
                0x1FFF_FFF8,
                "page {index_page} empty slot"
            );
        }
    }

    #[test]
    fn fixed_schema_pages_carry_their_magic_values() {
        let (tracks, playlists) = small_export();
        let bytes = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");

        // (page, table, rows, sequence, unknown5, num_rows_large, flags)
        let expected = [
            (KEYS_DATA_PAGE, T_KEYS, 24u16, 0x1Bu32, 1u16, 23u16, 0x24u8),
            (COLORS_DATA_PAGE, T_COLORS, 8, 2, 8, 0, 0x24),
            (COLUMNS_DATA_PAGE, T_COLUMNS, 27, 3, 27, 0, 0x24),
            (MENU_DATA_PAGE, T_MENU, 22, 4, 22, 0, 0x24),
            (UNKNOWN_12_DATA_PAGE, T_UNKNOWN_12, 17, 5, 17, 0, 0x24),
            (HISTORY_DATA_PAGE, T_HISTORY, 1, 1, 1, 0, 0x24),
        ];
        for (page_index, table_type, rows, sequence, unknown5, num_rows_large, flags) in expected {
            let page = &bytes[page_index as usize * PAGE_SIZE..][..PAGE_SIZE];
            assert_eq!(
                u32_at(page, 0x04),
                page_index,
                "page {page_index} self index"
            );
            assert_eq!(u32_at(page, 0x08), table_type, "page {page_index} table");
            assert_eq!(u32_at(page, 0x10), sequence, "page {page_index} sequence");
            assert_eq!(u16_at(page, 0x20), unknown5, "page {page_index} unknown5");
            assert_eq!(
                u16_at(page, 0x22),
                num_rows_large,
                "page {page_index} num_rows_large"
            );
            assert_eq!(page[0x1B], flags, "page {page_index} flags");
            assert_eq!(
                packed_row_counts(u32::from(rows), u32::from(rows)),
                [page[0x18], page[0x19], page[0x1A]]
            );
        }

        // The fixed browse menu and table 0x12 rows are constants.
        let menu = &bytes[MENU_DATA_PAGE as usize * PAGE_SIZE..][..PAGE_SIZE];
        for (index, (category, content, kind, visibility, sort_order)) in
            MENU_ROWS.iter().enumerate()
        {
            let base = HEAP_START + index * 8;
            assert_eq!(u16_at(menu, base), *category);
            assert_eq!(u16_at(menu, base + 2), *content);
            assert_eq!(menu[base + 4], *kind);
            assert_eq!(menu[base + 5], *visibility);
            assert_eq!(u16_at(menu, base + 6), *sort_order);
        }
        let table_12 = &bytes[UNKNOWN_12_DATA_PAGE as usize * PAGE_SIZE..][..PAGE_SIZE];
        for (index, (first, second, third)) in UNKNOWN_12_ROWS.iter().enumerate() {
            let base = HEAP_START + index * 8;
            assert_eq!(u16_at(table_12, base), *first);
            assert_eq!(u16_at(table_12, base + 2), *second);
            assert_eq!(u32_at(table_12, base + 4), *third);
        }
    }

    #[test]
    fn many_rows_overflow_into_extra_pages() {
        // Well past one page of track rows and one page of playlist entries.
        let tracks: Vec<CdjTrack> = (1..=200)
            .map(|index| {
                track(
                    &format!("Track {index:03}"),
                    &format!("Artist {}", index % 7),
                    &format!("Album {}", index % 5),
                    Some("House"),
                )
            })
            .collect();
        let playlists = vec![CdjPlaylist {
            name: "Everything".to_owned(),
            entries: (0..tracks.len()).collect(),
        }];
        let bytes = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");

        assert!(bytes.len() > STANDARD_PAGES * PAGE_SIZE);
        // Overflow pages start after the reserved zone and are shared out
        // between the tables that need them.
        assert!(bytes.len() >= (OVERFLOW_START as usize + 1) * PAGE_SIZE);
        assert_eq!(rows_of::<Track>(&bytes).len(), tracks.len());
        assert_eq!(rows_of::<PlaylistEntry>(&bytes).len(), tracks.len());

        // Table 0 (tracks) is the first pointer in page 0.
        let tracks_last_page = u32_at(&bytes, 0x1C + 12);
        assert!(
            tracks_last_page >= OVERFLOW_START,
            "more than one tracks data page"
        );
        assert!(u32_at(&bytes, 0x0C) > tracks_last_page, "next_unused_page");
    }

    #[test]
    fn oversized_row_is_rejected() {
        let mut broken = track("Big", "Artist", "Album", None);
        broken.device_audio_path = format!("/Contents/{}", "x".repeat(PAGE_SIZE));
        assert!(matches!(
            build(&[broken], &[], &[], EXPORT_DATE),
            Err(PdbError::RowTooLarge {
                table: "tracks",
                ..
            })
        ));

        // A history row that cannot fit is rejected too, rather than panicking.
        let long_date = "1".repeat(PAGE_SIZE);
        assert!(matches!(
            build(&[], &[], &[], &long_date),
            Err(PdbError::RowTooLarge {
                table: "history",
                ..
            })
        ));
    }

    #[test]
    fn identical_input_produces_identical_bytes() {
        let (tracks, playlists) = small_export();
        let first = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");
        let second = build(&tracks, &playlists, &[], EXPORT_DATE).expect("build pdb");
        assert_eq!(first, second);
    }

    #[test]
    fn unknown_bpm_and_key_fall_back_to_zero() {
        let mut track = track("Odd", "Artist", "Album", None);
        track.bpm = Some(f64::NAN);
        track.key_id = Some(99);
        let bytes = build(&[track], &[], &[], EXPORT_DATE).expect("build pdb");

        let parsed = rows_of::<Track>(&bytes);
        let json = serde_json::to_value(&parsed[0]).expect("serialize row");
        assert_eq!(json["tempo"], 0);
        assert_eq!(json["key_id"], 0);
        assert_eq!(json["genre_id"], 0, "no genre row exists");
        assert_eq!(json["file_type"], "Mp3");
    }

    #[test]
    fn non_ascii_metadata_round_trips() {
        let mut track = track("Café Noir", "Émile", "Été", Some("Électronique"));
        track.file_type = CdjFileType::Flac;
        let bytes = build(&[track], &[], &[], EXPORT_DATE).expect("build pdb");

        let parsed = rows_of::<Track>(&bytes);
        assert_eq!(
            parsed[0].offsets.title.clone().into_string().unwrap(),
            "Café Noir"
        );
        let json = serde_json::to_value(&parsed[0]).expect("serialize row");
        assert_eq!(json["file_type"], "Flac");
        assert_eq!(
            json["offsets"]["inner"]["file_path"],
            "/Contents/Émile/Été/Café Noir.mp3"
        );

        let albums = rows_of::<Album>(&bytes);
        let album = serde_json::to_value(&albums[0]).expect("serialize row");
        assert_eq!(album["offsets"]["inner"]["name"], "Été");

        let genres = rows_of::<Genre>(&bytes);
        let genre = serde_json::to_value(&genres[0]).expect("serialize row");
        assert_eq!(genre["name"], "Électronique");
    }

    #[test]
    fn every_file_type_the_model_can_carry_is_written() {
        let types = [
            (CdjFileType::Mp3, "Mp3"),
            (CdjFileType::M4a, "M4a"),
            (CdjFileType::Flac, "Flac"),
            (CdjFileType::Wav, "Wav"),
            (CdjFileType::Aiff, "Aiff"),
            (CdjFileType::Unknown, "Unknown"),
        ];
        for (file_type, name) in types {
            let mut input = track("Type", "Artist", "Album", None);
            input.file_type = file_type;
            let bytes = build(&[input], &[], &[], EXPORT_DATE).expect("build pdb");
            let parsed = rows_of::<Track>(&bytes);
            let json = serde_json::to_value(&parsed[0]).expect("serialize row");
            assert_eq!(json["file_type"], name);
        }
    }
}
