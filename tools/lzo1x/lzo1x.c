/* lzo1x -- stdin/stdout raw LZO1X-1 compressor for OpenSpace payloads
 *
 * This file is CC0 1.0 (see repository LICENSE). Binaries built from it may be
 * GPL-covered when linked with minilzo from lib/lzo; see tools/lzo1x/LICENSE.
 *
 * Uses the vendored LZO 1.08 minilzo sources in lib/lzo. Build with -O0 so
 * output matches the legacy non-optimized encoder used by the original toolchain.
 *
 * Exit codes:
 *   0  compressed bytes written to stdout
 *   1  incompressible (output would not be smaller than input)
 *   2  usage / initialization / I/O error
 */

#include <errno.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "minilzo.h"

#define CHUNK_SIZE (256 * 1024)

static int read_all_stdin(unsigned char **out_data, size_t *out_len)
{
    size_t capacity = CHUNK_SIZE;
    size_t length = 0;
    unsigned char *buffer = (unsigned char *)malloc(capacity);

    if (buffer == NULL)
    {
        return -1;
    }

    for (;;)
    {
        if (length == capacity)
        {
            size_t new_capacity = capacity + CHUNK_SIZE;
            unsigned char *grown = (unsigned char *)realloc(buffer, new_capacity);
            if (grown == NULL)
            {
                free(buffer);
                return -1;
            }

            buffer = grown;
            capacity = new_capacity;
        }

        size_t space = capacity - length;
        size_t read_count = fread(buffer + length, 1, space, stdin);
        if (read_count == 0)
        {
            if (ferror(stdin))
            {
                free(buffer);
                return -1;
            }

            break;
        }

        length += read_count;
    }

    *out_data = buffer;
    *out_len = length;
    return 0;
}

int main(void)
{
    unsigned char *input = NULL;
    size_t in_len = 0;
    unsigned char *output = NULL;
    lzo_uint out_len = 0;
    int status = 2;

    if (lzo_init() != LZO_E_OK)
    {
        fprintf(stderr, "lzo1x: lzo_init failed\n");
        return 2;
    }

    if (read_all_stdin(&input, &in_len) != 0)
    {
        fprintf(stderr, "lzo1x: failed to read stdin: %s\n", strerror(errno));
        goto cleanup;
    }

    if (in_len == 0)
    {
        status = 0;
        goto cleanup;
    }

    {
        lzo_uint max_out = (lzo_uint)in_len + (lzo_uint)(in_len / 16) + 64 + 3;
        output = (unsigned char *)malloc(max_out);
        if (output == NULL)
        {
            fprintf(stderr, "lzo1x: out of memory\n");
            goto cleanup;
        }

        {
            lzo_align_t __LZO_MMODEL wrkmem[((LZO1X_1_MEM_COMPRESS) + (sizeof(lzo_align_t) - 1)) / sizeof(lzo_align_t)];
            int r = lzo1x_1_compress(input, (lzo_uint)in_len, output, &out_len, wrkmem);
            if (r != LZO_E_OK)
            {
                fprintf(stderr, "lzo1x: compression failed (%d)\n", r);
                goto cleanup;
            }
        }

        if (out_len >= in_len)
        {
            status = 1;
            goto cleanup;
        }

        if (fwrite(output, 1, out_len, stdout) != out_len || fflush(stdout) != 0)
        {
            fprintf(stderr, "lzo1x: failed to write stdout: %s\n", strerror(errno));
            goto cleanup;
        }

        status = 0;
    }

cleanup:
    free(output);
    free(input);
    return status;
}