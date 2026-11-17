// The game's own subsdk modules' places in guest memory (gamesub.h).
#include "gamesub.h"

#include <stdio.h>
#include <sys/stat.h>

#include "modlib.h"
#include "rt.h"

#define ALIGN_UP(v, a) (((v) + (a) - 1) & ~((a) - 1))

int gamesub_layout(const char *exefs, GameSub *out, uint64_t *end, char *why, size_t why_size) {
    char mp[1100];
    snprintf(mp, sizeof mp, "%s/main", exefs);
    uint64_t at = ALIGN_UP(MAIN_BASE + nso_image_size(mp), MOD_ALIGN);
    int n = 0;
    for (int k = 0; k < GAMESUB_MAX; k++) {
        GameSub *s = &out[n];
        snprintf(s->name, sizeof s->name, "subsdk%d", k);
        snprintf(s->path, sizeof s->path, "%s/%s", exefs, s->name);
        struct stat st;
        if (stat(s->path, &st)) continue;  // (subsdk0..3, gaps allowed: the console loads what is there)
        s->size = nso_image_size(s->path);
        if (!s->size) {
            if (why) snprintf(why, why_size, "the game files' exefs/%s is unreadable", s->name);
            return -1;
        }
        s->base = at;
        at = ALIGN_UP(at + s->size, MOD_ALIGN);
        if (at > SDK_BASE) {  // (main + subsdks share the 128 MB below the SDK: Arceus 1.0.1 ends at ~86 MB)
            if (why) snprintf(why, why_size, "the game's modules don't fit below the SDK (%s ends at 0x%llx)", s->name,
                              (unsigned long long)at);
            return -1;
        }
        n++;
    }
    if (end) *end = at;
    return n;
}
