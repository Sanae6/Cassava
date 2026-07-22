use num_derive::{FromPrimitive, ToPrimitive};
use num_traits::FromPrimitive;
use snafu::{OptionExt, Snafu, ensure};

use crate::{nw::{bffnt::Error::InvalidByteOrderMark, bntx::TextureArchive}, util::{BinaryReader, ByteOrder}};

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(transparent)]
	ReadError {
		source: crate::util::Error,
		backtrace: snafu::Backtrace,
	},
    UnsupportedVersion,
	#[snafu(display("expected magic to be 'FFNT'"))]
	IncorrectHeaderMagic,
    #[snafu(display("expected magic to be 'FINF'"))]
	IncorrectFontInfoMagic,
    #[snafu(display("expected magic to be 'TGLP'"))]
	IncorrectTextureGlyphMagic,
    #[snafu(display("expected magic to be 'CMAP'"))]
	IncorrectCodeMapMagic,
    InvalidByteOrderMark,
    IncorrectHeaderSize,
    InvalidVersion,
    InvalidCharacterCode,
    InvalidFontType,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum CharacterCode {
    Unicode = 1,
    ShiftJIS = 2,
    CP1252 = 3,
}

#[derive(FromPrimitive, ToPrimitive, Debug, Clone, Copy, PartialEq, Eq)]
pub enum FontType {
    Glyph = 0,
    Texture = 1,
    PackedTexture = 2
}

#[derive(Debug)]
pub struct TextureGlyph {
    cell_width: u8,
    cell_height: u8,
    maximum_character_width: u8,
    baseline_position: i16,
    rows: u16,
    lines: u16,
    archive: TextureArchive,
}

impl TextureGlyph {
    pub fn read(reader: &mut BinaryReader) -> Result<Self, Error> {
        todo!()
    }
}

#[derive(Debug)]
pub struct Font {
    font_type: FontType,
    height: u8,
    width: u8,
    ascent: u8,
    line_feed: i16,
    alter_character_index: u16,
    default_left_width: i8,
    default_glyph_width: u8,
    default_character_width: u8,
    character_code: CharacterCode,
    texture_glyph: TextureGlyph

}

impl Font {
    pub fn read(reader: &mut BinaryReader) -> Result<Self, Error> {

        let magic = reader.read_bytes::<4>()?;
        ensure!(&magic != b"FFNT", IncorrectHeaderMagicSnafu);

        let byte_order = reader.read_bytes::<2>()?;
        match byte_order {
            [0xFE, 0xFF] => reader.set_byte_order(ByteOrder::Big),
            [0xFF, 0xFE] => reader.set_byte_order(ByteOrder::Little),
            _ => return Err(InvalidByteOrderMark)
        }

        ensure!(reader.read_u16()? == 0x14, IncorrectHeaderSizeSnafu);

        ensure!(reader.read_u32()? == 0x04010000, InvalidVersionSnafu);

        reader.seek_rel(8)?;
        
        ensure!(&reader.read_bytes::<4>()? != b"FINF", IncorrectFontInfoMagicSnafu);

        ensure!(reader.read_u32()? == 0x20, IncorrectHeaderSizeSnafu);

        let font_type = FontType::from_u8(reader.read_u8()?).context(InvalidFontTypeSnafu)?;
        let height = reader.read_u8()?;
        let width = reader.read_u8()?;
        let ascent = reader.read_u8()?;
        let line_feed = reader.read_i16()?;
        let alter_character_index = reader.read_u16()?;
        let default_left_width = reader.read_i8()?;
        let default_glyph_width = reader.read_u8()?;
        let default_character_width = reader.read_u8()?;
        let character_code = CharacterCode::from_u8(reader.read_u8()?).context(InvalidCharacterCodeSnafu)?;

        // let font = Font {
        //     font_type,
        //     height,
        //     width,
        //     ascent,
        //     line_feed,
        //     alter_character_index,
        //     default_left_width,
        //     default_glyph_width,
        //     default_character_width,
        //     character_code
        // };

        todo!()
    }
}