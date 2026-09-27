/*
 * Minimal C example: parse a PDX file into its default UDS repro
 * sequence, print it, then do it again with a custom sequence file.
 *
 * Build and run via the provided Makefile: `make run`.
 */
#include <stdio.h>
#include "repro_toolkit.h"

static void print_sequence(const char *label, const char *pdx_path, const char *sequence_path) {
    char *json = repro_toolkit_generate_sequence(pdx_path, sequence_path);
    if (json == NULL) {
        fprintf(stderr, "%s: pdx_path was NULL\n", label);
        return;
    }

    printf("=== %s ===\n%s\n\n", label, json);
    repro_toolkit_free_string(json);
}

int main(int argc, char **argv) {
    if (argc < 2) {
        fprintf(stderr, "usage: %s <ecu.pdx> [custom-sequence.json]\n", argv[0]);
        return 1;
    }

    const char *pdx_path = argv[1];
    const char *sequence_path = argc >= 3 ? argv[2] : NULL;

    /* Default, auto-generated sequence. */
    print_sequence("default sequence", pdx_path, NULL);

    /* Custom sequence, if one was given on the command line. */
    if (sequence_path != NULL) {
        print_sequence("custom sequence", pdx_path, sequence_path);
    }

    return 0;
}
