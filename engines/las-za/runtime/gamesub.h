// SwitchOS: the game's own subsdk modules (exefs/subsdk0-3) - Pokémon Legends: Arceus ships two, Z-A none. On the
// console rtld maps them next to main; here they sit right after main's image, each aligned to MOD_ALIGN, and code
// mods follow them. The loader (loader.c) and the game code builder (gamebuild.c: gen.py lifts each one at its base)
// both take the places from gamesub_layout, so the two always agree.
// Their function tables come from the game code library's optional second export (GAMELIB_SUB_ENTRY), written by
// gamebuild.c next to tools/gamelib_entry.c: gamelib.h and the library ABI stay as they are, so adding this didn't
// change GAMELIB_ID (a Z-A player keeps the game code already built).
#pragma once
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "cpu.h"

#define GAMESUB_MAX 4
#define GAMELIB_SUB_ENTRY "lasgame_sub_tables"

typedef struct {
    char name[16];       // "subsdk0".."subsdk3"
    char path[1100];     // <exefs>/<name>
    uint64_t base, size; // guest address and image size (nso_image_size)
} GameSub;

// The library's subsdk tables, as gamebuild.c writes them (lasgame_sub_tables)
typedef struct {
    const char *name;
    const uint64_t *addrs;  // sorted guest addresses
    const GuestFn *fns;     // the functions at them
    const uint64_t *count;
} GameLibSub;
typedef struct {
    uint32_t n;
    GameLibSub mods[GAMESUB_MAX];
} GameLibSubTables;
typedef const GameLibSubTables *(*GameLibSubEntry)(void);

#ifdef __cplusplus
extern "C" {
#endif

// The subsdks in exefs (in name order) and their places. Returns how many (0 for Z-A), or -1 when they don't fit
// below SDK_BASE (why says so). *end: the first free address after them, where the code mods start.
int gamesub_layout(const char *exefs, GameSub *out, uint64_t *end, char *why, size_t why_size);
// The loaded game code library's subsdk tables (gamelib.c), NULL when it has none
const GameLibSubTables *gamelib_sub_tables(void);

#ifdef __cplusplus
}
#endif
