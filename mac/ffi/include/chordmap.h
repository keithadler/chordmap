// C face of chordmap (https://github.com/keithadler/chordmap) for Chordmap for Mac. See ffi/src/lib.rs.
#ifndef CHORDMAP_FFI_H
#define CHORDMAP_FFI_H
#include <stddef.h>
#include <stdint.h>
#ifdef __cplusplus
extern "C" {
#endif
char *chordmap_analyze(const float *samples, size_t n, uint32_t sample_rate, const char *options_json);
char *chordmap_chord_sheet(const char *analysis_json);
char *chordmap_version(void);
void chordmap_free(char *p);
float *chordmap_synth(const char *chords, float bpm, size_t loops, size_t *out_len);
void chordmap_free_samples(float *p, size_t n);
uint32_t chordmap_sample_rate(void);
#ifdef __cplusplus
}
#endif
#endif
