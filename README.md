# Facewave

A lossy compression codec for faces written in Rust. IIt exploits facial symmetry by splitting each image into a symmetric and an antisymmetric channel. Then compressing both independently with a Haar wavelet transform, EZW zerotree coding and adaptive arithmetic coding, into a custom `.face` format.

It's aimed at storing faces in bulk at very small sizes, while keeping them recognisable.

| bpp  | Size   | Compression | PSNR    | JPEG, similar size      | Gain    |
|-----:|-------:|------------:|--------:|------------------------:|--------:|
| 0.29 | 593 B  | 27.6×       | 30.4 dB | 28.8 dB (q7, 551 B)     | +1.6 dB |
| 0.52 | 1060 B | 15.5×       | 35.8 dB | 34.1 dB (q24, 1057 B)   | +1.7 dB |
| 1.00 | 2054 B | 8.0×        | 41.2 dB | 39.8 dB (q73, 2031 B)   | +1.5 dB |
| 1.76 | 3613 B | 4.5×        | 49.7 dB | 46.0 dB (q92, 3413 B)   | +3.7 dB |

Measured on a single 128×128 greyscale test face, after alignment; JPEG was run
on the same aligned image. Compression is relative to raw 8-bit (16,384 B).

128×128 greyscale test face; compression is relative to raw 8-bit (16,384 B).

## Running it

BUILD BASIC CLI

## Architecture

<img width="292" height="652" alt="Untitled Diagram drawio" src="https://github.com/user-attachments/assets/4113130e-d933-45e6-859a-91963aea6fc2" />

## Results

## References

[1] J. M. Shapiro, Embedded Image Coding Using Zerotrees of Wavelet Coefficients, IEEE Transactions on Signal Processing, 41(12), 1993.

[2] I. H. Witten, R. M. Neal and J. G. Cleary, Arithmetic Coding for Data Compression, Communications of the ACM, 30(6), 1987.
