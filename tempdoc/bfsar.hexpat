import std.io;
import std.mem;
import std.core;
import type.base;

using SectionInfo;
using StringSection;
using InfoSection;
using FileSection;
struct SoundArchive {
    char magic[4]; // FSAR
    u8 bom[2]; // [0xFF, 0xFE] on little endian
    u16 headerSize; // header aligned to 0x10 bytes
    u8 revision;
    u8 minor;
    u16 major;
    u32 fileSize;
    u16 sectionCount;
    padding[2];
    SectionInfo sectionInfo[sectionCount];
    std::mem::AlignTo<16> align;
};

struct SectionInfo {
    u16 type;
    padding[2];
    u32 offset;
    u32 size;
};
SoundArchive arc @ 0;

fn find_section(u16 type) {
    auto offset = 0;
    for (auto i = 0, i < arc.sectionCount, i += 1)
        if (arc.sectionInfo[i].type == type)
            offset = arc.sectionInfo[i].offset;
    return offset;
};

u32 stringTableStart;
struct String {
    // poor man's for-loop
    char data[] @ stringTableStart + parent.entries[std::core::array_index()].offset;
};
using StringTable;
using StringTableEntry;
struct StringSection {
    u32 sectionStart = $ + 8;
    char magic[4];
    u32 sectionSize;
    u16 stringTableFlag;
    padding[2];
    u32 stringTableOffset;
    u16 lookupTableFlag;
    padding[2];
    u32 lutOffset;
    stringTableStart = sectionStart + stringTableOffset;
    StringTable table @ stringTableStart;
    StringTableEntry entries[table.entryCount] @ stringTableStart + 4;
    String strings[table.entryCount];
};
struct StringTableEntry {
    u16 flag;
    padding[2];
    u32 offset;
    u32 size;
};
struct StringTable {
    u32 entryCount;
};
StringSection string @ find_section(0x2000);
struct FileSection {
    char magic[4]; // FILE
    u32 size;
    u8 data[size - 8]; // unstructured blob, see INFO for offsets
};
FileSection file @ find_section(0x2002);

u32 infoStart = 0;
struct Reference {
    type::Hex<u16> typeId;
    padding[2];
    u32 offset;
};
struct ReferenceTable {
    $ = infoStart + parent.references[std::core::array_index()].offset;
    std::print("table at {:X}", $);
    u32 count;
    Reference references[count];
};
struct InfoSection {
    char magic[4];
    u32 sectionSize;
    infoStart = $;
    std::print("{:X}", infoStart);
    Reference references[8];
    ReferenceTable tables[1];
};

InfoSection info @ find_section(0x2001);