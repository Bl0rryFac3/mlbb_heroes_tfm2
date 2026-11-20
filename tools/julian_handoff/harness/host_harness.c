/*
 * Minimal stand-in for the TFM2 stable loader.
 *
 * Loads a stable-ABI mod DLL, calls its real exported tfm2_mod_entry_stable()
 * with a size-correct, all-null-slot HostApiV1 (ABI level 10, game 0.7.2),
 * then walks the returned ModExportV1 and prints the registry names.
 *
 * It proves: the PE loads on Windows, both stable entry exports resolve, the
 * mod's init path runs to completion and returns an export, and the named
 * native effects/passives a specific hero needs are actually registered.
 *
 * It is NOT the game. No simulation, combat, rendering, audio, save, AI or
 * asset pipeline is exercised; all host slots are null, exactly as the size-
 * guarded SDK intends for an older/limited host.
 *
 * Struct layouts mirror sdk/mod-api-stable at ABI_LEVEL 10. No project code
 * was modified to make this run.
 */
#include <windows.h>
#include <stdio.h>
#include <string.h>

typedef unsigned int u32;
typedef unsigned long long usize_t;

typedef struct { u32 major, minor, patch; } GameVersionV1;
typedef struct { const unsigned char *ptr; usize_t len; } StrV1;
typedef struct { void *userdata; const void *vtable; } RegPairV1;

typedef struct { StrV1 name; RegPairV1 effect; } NativeEffectRegV1;
typedef struct { StrV1 name; RegPairV1 passive; } NativePassiveRegV1;

typedef struct {
    usize_t size;
    u32 host_abi_level;
    GameVersionV1 game_version;
    void *log;
    const void *sim;
    const void *frame;
    const void *service;
    const void *save;
    const void *net;
    const void *data;
    const void *ui;
    const void *scene;
    const void *asset;
} HostApiV1;

typedef struct {
    usize_t size;
    u32 required_abi_level;
    const unsigned char *mod_id_ptr;
    usize_t mod_id_len;
    void *destroy;
    const RegPairV1 *champions; usize_t champions_len;
    const RegPairV1 *items; usize_t items_len;
    const NativeEffectRegV1 *native_effects; usize_t native_effects_len;
    RegPairV1 server_ext;
    RegPairV1 extension;
    const RegPairV1 *draft_hooks; usize_t draft_hooks_len;
    const RegPairV1 *player_ai; usize_t player_ai_len;
    RegPairV1 map_customizer;
    RegPairV1 match_hook;
    const RegPairV1 *item_build_hooks; usize_t item_build_hooks_len;
    const NativePassiveRegV1 *native_passives; usize_t native_passives_len;
} ModExportV1;

typedef u32 (__cdecl *abi_fn)(void);
typedef ModExportV1 *(__cdecl *entry_fn)(const HostApiV1 *);

static int saw(const char *name, usize_t len, const char *want) {
    usize_t m = (usize_t)strlen(want);
    return m <= len && memcmp(name, want, (size_t)m) == 0 && m == len;
}

int main(int argc, char **argv) {
    if (argc < 2) { printf("usage: host_harness <dll> [required registry name ...]\n"); return 2; }
    HMODULE h = LoadLibraryA(argv[1]);
    if (!h) { printf("FAIL LoadLibrary error=%lu\n", (unsigned long)GetLastError()); return 1; }
    printf("OK   LoadLibrary\n");

    abi_fn abi = (abi_fn)(void *)GetProcAddress(h, "tfm2_mod_required_abi_level");
    entry_fn entry = (entry_fn)(void *)GetProcAddress(h, "tfm2_mod_entry_stable");
    if (!abi || !entry) { printf("FAIL missing stable entry exports\n"); return 1; }
    u32 level = abi();
    printf("OK   tfm2_mod_required_abi_level() = %u\n", level);
    if (level > 10) { printf("FAIL required ABI level %u exceeds harness host level 10\n", level); return 1; }

    HostApiV1 host;
    memset(&host, 0, sizeof host);
    host.size = sizeof host;
    host.host_abi_level = 10;
    host.game_version.major = 0;
    host.game_version.minor = 7;
    host.game_version.patch = 2;

    ModExportV1 *ex = entry(&host);
    if (!ex) { printf("FAIL entry returned NULL export (init failed or panicked)\n"); return 1; }
    printf("OK   entry returned export: size=%llu required_abi=%u\n",
           (unsigned long long)ex->size, ex->required_abi_level);
    if (ex->mod_id_ptr && ex->mod_id_len)
        printf("OK   mod_id = \"%.*s\"\n", (int)ex->mod_id_len, (const char *)ex->mod_id_ptr);

    printf("INFO champions=%llu items=%llu native_effects=%llu native_passives=%llu\n",
           (unsigned long long)ex->champions_len, (unsigned long long)ex->items_len,
           (unsigned long long)ex->native_effects_len,
           (unsigned long long)ex->native_passives_len);

    int julian_effects = 0, julian_passives = 0;
    printf("INFO effects:");
    for (usize_t i = 0; i < ex->native_effects_len; ++i) {
        const NativeEffectRegV1 *e = &ex->native_effects[i];
        if (!e->name.ptr) continue;
        if (e->name.len > 9 && memcmp(e->name.ptr, "mlbb_heroes:julian", 18) == 0) {
            ++julian_effects;
            printf(" %.*s", (int)e->name.len, (const char *)e->name.ptr);
        }
    }
    printf("\nINFO julian_passives:");
    for (usize_t i = 0; i < ex->native_passives_len; ++i) {
        const NativePassiveRegV1 *p = &ex->native_passives[i];
        if (!p->name.ptr) continue;
        if (p->name.len >= 18 && memcmp(p->name.ptr, "mlbb_heroes:julian", 18) == 0) {
            ++julian_passives;
            printf(" %.*s", (int)p->name.len, (const char *)p->name.ptr);
        }
    }
    printf("\nINFO julian registered: %d effects, %d passives\n", julian_effects, julian_passives);

    int rc = 0;
    for (int a = 2; a < argc; ++a) {
        const char *want = argv[a];
        int found = 0;
        for (usize_t i = 0; i < ex->native_effects_len && !found; ++i) {
            const NativeEffectRegV1 *e = &ex->native_effects[i];
            if (e->name.ptr) found = saw((const char *)e->name.ptr, e->name.len, want);
        }
        for (usize_t i = 0; i < ex->native_passives_len && !found; ++i) {
            const NativePassiveRegV1 *p = &ex->native_passives[i];
            if (p->name.ptr) found = saw((const char *)p->name.ptr, p->name.len, want);
        }
        if (!found) { printf("FAIL required registration '%s' missing\n", want); rc = 1; }
        else printf("OK   registered '%s'\n", want);
    }
    printf("RESULT %s\n", rc ? "FAIL" : "PASS");
    return rc;
}
