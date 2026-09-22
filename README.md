# Facewave

A lossy compression codec for faces written in Rust. It exploits facial symmetry by splitting each image into a symmetric and an antisymmetric channel. Then compressing both independently with a Haar wavelet transform, EZW zerotree coding and adaptive arithmetic coding, into a custom `.face` format.

It's aimed at storing faces in bulk at very small sizes, while keeping them recognisable.

| bpp  | Size   | Compression | PSNR    | JPEG, similar size      | Gain    |
|-----:|-------:|------------:|--------:|------------------------:|--------:|
| 0.29 | 593 B  | 27.6×       | 30.4 dB | 28.8 dB (q7, 551 B)     | +1.6 dB |
| 0.52 | 1060 B | 15.5×       | 35.8 dB | 34.1 dB (q24, 1057 B)   | +1.7 dB |
| 1.00 | 2054 B | 8.0×        | 41.2 dB | 39.8 dB (q73, 2031 B)   | +1.5 dB |
| 1.76 | 3613 B | 4.5×        | 49.7 dB | 46.0 dB (q92, 3413 B)   | +3.7 dB |

Measured on a single 128×128 greyscale test face, after alignment; JPEG was run
on the same aligned image. Compression is relative to raw 8-bit (16,384 B).

## Running it

Build:
```
cargo build --release
```

Compress an image and decompress it again:
```
./target/release/facewave encode face.jpg -o face.face
./target/release/facewave decode face.face -o face.png
```

Quality is set with `-q` from 1 (smallest) to 10 (best), default 6. Each level
is a preset of transform depth and quantisation step for the two channels,
taken from the sweep's Pareto front. Individual values can be overridden:
```
./target/release/facewave encode face.jpg -o face.face -q 8
./target/release/facewave encode face.jpg -o face.face --levels-s 4 --step-s 0.125
```


## Architecture

At a high level: an image is converted to greyscale, aligned on its symmetry axis and resized to 128×128, then split into a symmetric channel S and an antisymmetric channel D. Each channel is then compressed independently. This starts with a Discrete Wavelet Transform (DWT) to first concentrate the energy into a few large coefficients, then a quantisation step to discard precision. And then we perform EZW zerotree coding to exploit this positional information the DWT left us and then an adaptive arithmetic coder the squeeze the remaining redundancy out of the symbol stream. Both channels are packed into a single `.face` file.

<p align="center">
  <br>
  <img width="292" alt="facewave encode pipeline" src="https://github.com/user-attachments/assets/4113130e-d933-45e6-859a-91963aea6fc2" />
  <br>
  <em>The encode pipeline. Decoding runs the same path in reverse.</em>
  <br><br>
</p>

Decoding runs the same path in reverse. Every stage except quantisation is exactly invertible, so quantisation is the only source of loss.

The wavelet transform and the zerotree coder follow Shapiro [1], and the arithmetic coder follows Witten, Neal and Cleary [2]. The symmetry split, the alignment step and the `.face` format are specific to this project.


### Preprocessing: greyscale, alignment, resize

The first stage is **greyscale**, this was done for 2 reasons: it saves space, and faces are still recognisable with greyscale.

The next stage was **alignment**. Facewave performs well because it exploits facial symmetry, it splits the face into symmetric information (S) and asymmetric information (D). However most faces are rarely centred. If the axis is even a few pixels off, every pixel is paired with the wrong partner, so D fills up with real structure rather than staying near zero. Alignment solves this by shifting the axis to minimise $\lVert D\rVert_F^2$. This is done by searching exhaustively the space since it is small.

Finally **resize** was performed. Since the DWT halves dimension at every level, having a power of 2 size was important. So I chose 128×128. Having a consistent size also made bpp figures comparable across images.

### The symmetry split

To exploit symmetry we hold the symmetric information (S) and asymmetric information (D) separately for some image *I* and its mirrored version *flip(I)*.

$$S = \frac{I + \mathrm{flip}(I)}{\sqrt 2}, \qquad D = \frac{I - \mathrm{flip}(I)}{\sqrt 2}$$


<p align="center">
  <img width="96" hspace="20" alt="symmetric channel S" src="https://github.com/user-attachments/assets/ea12f9e0-6feb-4fc9-afa6-1d4be1a90f7d" />
  <img width="96" hspace="20" alt="antisymmetric channel D" src="https://github.com/user-attachments/assets/dec665a3-fa80-425c-9cae-2fcbb7a8d3a1" />
  <br>
  <em>Left: the symmetric channel S.</em>
  <br>
  <em>Right: the antisymmetric channel D, near zero almost everywhere.</em>
  <br>
  <em>Both are rescaled for display: S by 1/√2, and D with zero mapped to mid-grey.</em>
</p>

We define energy as $\lVert I\rVert^2$. This represents how much signal we must encode. The reason for dividing earlier by $\sqrt{2}$ followed from the conservation of energy: $\lVert I\rVert^2 = \lVert S\rVert^2 + \lVert D\rVert^2$.

An interesting result we get from the split if that on this test image D holds 0.069% of the total energy, and 0.46% when we remove the constant brightness term.

### Haar wavelet transform

Each channel is put through a multi-level 2D Haar transform. Each level leaves four quadrants: a half-size approximation of the image (LL) and three detail bands holding the vertical, horizontal and diagonal edges it discarded (LH, HL, HH). The next level repeats this on LL alone, so the transform recurses into the approximation and leaves the detail bands untouched.

<p align="center">
  <img width="96" alt="Haar pyramid of the antisymmetric channel" src="https://github.com/user-attachments/assets/95805068-ad45-46f8-aeaa-5384b48cec60" />
  <br>
  <em>The antisymmetric channel after four levels of the Haar transform: the approximation shrinks into the top-left corner and the detail bands fill the rest, mostly near zero.</em>
</p>

The reason we do this is to compact the energy into the corner and leave the rest more sparse. This allows our quantiser to exploit near zero coefficients. And similar to the split, energy is conserved with this transformation.

### Quantisation

Each coefficient is divided by a step size and rounded to the nearest integer. This is where where we exploit sparsity by rounding lots of values to zero. Every band has its own step, so coarse and fine detail can be traded off independently, and each channel has its own steps too.

This is the only lossy stage, everything before it is orthonormal and everything after it is exactly reversible, so the step sizes alone set the trade-off between size and quality.


### EZW zerotree coding

Quantisation leaves mostly zeros, so the cost we face is now not what values the coefficients are, but where the nonzero ones are. We can save a lot of data by using the fact that zeros aren't scattered at random. Each coefficient in a detail band has four children in the band one level finer, covering the same patch of the image, and a region that is smooth enough to be zero at a coarse level is almost always smooth at finer levels too. So zeros appear in whole subtrees. EZW turns that into a single symbol.


<p align="center">
  <img width="373" height="360" alt="image" src="https://github.com/user-attachments/assets/8a6a1100-b400-4b87-a81b-79dab4162a04" />
  <br>
  <em>Parent–child links across subbands. Fig. 3 from Shapiro [1], © 1993 IEEE.</em>
</p>

Coding runs multiple times, each time halving the threshold of what we consider significant. Each run we do a *dominant pass* where we scan the coefficients and mark each coefficient as significant (with its sign), an isolated zero, or a zerotree root. We also run a *subordinate pass* that sends one more bit of magnitude for everything already significant

The algorithm follows Shapiro [1].


### Arithmetic coding

The EZW passes leave a stream of symbols with very uneven statistics, so the last stage squeezes out that redundancy. An arithmetic coder represents the whole message as a single number in $[0, 1)$, narrowing the interval to the slice each symbol owns.

The coder follows Witten, Neal and Cleary [2].

## References

[1] J. M. Shapiro, Embedded Image Coding Using Zerotrees of Wavelet Coefficients, IEEE Transactions on Signal Processing, 41(12), 1993.

[2] I. H. Witten, R. M. Neal and J. G. Cleary, Arithmetic Coding for Data Compression, Communications of the ACM, 30(6), 1987.
