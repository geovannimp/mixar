//! C ABI shim over Mixxx's vendored qm-dsp (beats + key).
//!
//! Mirrors `mixxx::AnalyzerQueenMaryBeats` (complex-domain detection function +
//! `TempoTrackV2`) and `mixxx::AnalyzerQueenMaryKey` (`GetKeyMode`), including
//! Mixxx's `DownmixAndOverlapHelper` framing (centred first window, zero-padded
//! finalize).

#include <cstdlib>
#include <vector>

#include "dsp/keydetection/GetKeyMode.h"
#include "dsp/onsets/DetectionFunction.h"
#include "dsp/tempotracking/TempoTrackV2.h"
#include "maths/MathUtilities.h"

namespace {

// Mirrors mixxx::DownmixAndOverlapHelper: fill a `windowSize` buffer, emit on
// full, shift left by `stepSize`; the first window is centred.
typedef void (*WindowCallback)(void* ctx, double* window, int windowSize);

void overlapFeed(const float* mono, long n, int windowSize, int stepSize,
                 void* ctx, WindowCallback callback) {
    if (windowSize <= 0 || stepSize <= 0 || stepSize > windowSize) {
        return;
    }
    std::vector<double> buffer((size_t)windowSize, 0.0);
    long writePos = (long)windowSize / 2;
    long i = 0;
    while (i < n) {
        long writeAvailable = (long)windowSize - writePos;
        long numFrames = (n - i) < writeAvailable ? (n - i) : writeAvailable;
        for (long k = 0; k < numFrames; ++k) {
            buffer[(size_t)(writePos + k)] = (double)mono[i + k];
        }
        writePos += numFrames;
        i += numFrames;
        if (writePos == (long)windowSize) {
            callback(ctx, &buffer[0], windowSize);
            for (long k = 0; k < (long)windowSize - stepSize; ++k) {
                buffer[(size_t)k] = buffer[(size_t)(k + stepSize)];
            }
            writePos -= stepSize;
        }
    }
    // finalize: append silence to complete the last window.
    long framesToFill = (long)windowSize - writePos;
    long minTail = (long)windowSize / 2 - 1;
    long need = framesToFill > minTail ? framesToFill : minTail;
    long done = 0;
    while (done < need) {
        long writeAvailable = (long)windowSize - writePos;
        long numFrames = (need - done) < writeAvailable ? (need - done) : writeAvailable;
        for (long k = 0; k < numFrames; ++k) {
            buffer[(size_t)(writePos + k)] = 0.0;
        }
        writePos += numFrames;
        done += numFrames;
        if (writePos == (long)windowSize) {
            callback(ctx, &buffer[0], windowSize);
            for (long k = 0; k < (long)windowSize - stepSize; ++k) {
                buffer[(size_t)k] = buffer[(size_t)(k + stepSize)];
            }
            writePos -= stepSize;
        }
    }
}

struct DfCtx {
    DetectionFunction* df;
    std::vector<double>* results;
    int frameLength;
};

void dfCallback(void* ctx, double* window, int windowSize) {
    (void)windowSize;
    DfCtx* c = static_cast<DfCtx*>(ctx);
    c->results->push_back(c->df->processTimeDomain(window));
}

struct KeyCtx {
    GetKeyMode* keyMode;
    int counts[25];
};

void keyCallback(void* ctx, double* window, int windowSize) {
    (void)windowSize;
    KeyCtx* c = static_cast<KeyCtx*>(ctx);
    int k = c->keyMode->process(window);
    if (k >= 1 && k <= 24) {
        c->counts[k] += 1;
    }
}

}  // namespace

extern "C" {

// Beat positions in samples (as Mixxx stores them). Returns the count; the
// caller-provided pointer receives a malloc'd array (free with qm_free).
int qm_beats(const float* mono, int n, int sr, double** beats_out) {
    *beats_out = 0;
    if (!mono || n <= 0 || sr <= 0) {
        return 0;
    }

    // mixxx/src/analyzer/plugins/analyzerqueenmarybeats.cpp
    const double kStepSecs = 0.01161;
    int stepSizeFrames = (int)((double)sr * kStepSecs);
    int windowSize = MathUtilities::nextPowerOfTwo(sr / 50);
    if (stepSizeFrames <= 0 || windowSize <= 0) {
        return 0;
    }

    DFConfig config;
    config.DFType = DF_COMPLEXSD;
    config.stepSize = stepSizeFrames;
    config.frameLength = windowSize;
    config.dbRise = 3;
    config.adaptiveWhitening = false;
    config.whiteningRelaxCoeff = -1;
    config.whiteningFloor = -1;

    DetectionFunction df(config);
    std::vector<double> results;
    DfCtx ctx;
    ctx.df = &df;
    ctx.results = &results;
    ctx.frameLength = windowSize;
    overlapFeed(mono, (long)n, windowSize, stepSizeFrames, &ctx, dfCallback);

    size_t nonZero = results.size();
    while (nonZero > 0 && results[nonZero - 1] <= 0.0) {
        --nonZero;
    }
    size_t required = (nonZero > 2 ? nonZero : 2) - 2;
    std::vector<double> d;
    d.reserve(required);
    for (size_t i = 2; i < nonZero; ++i) {
        d.push_back(results[i]);
    }

    std::vector<int> beatPeriod(required / 128 + 1);
    std::vector<double> beats;
    TempoTrackV2 tt((float)sr, stepSizeFrames);
    tt.calculateBeatPeriod(d, beatPeriod);
    tt.calculateBeats(d, beatPeriod, beats);

    int count = (int)beats.size();
    double* out = (double*)malloc(sizeof(double) * (size_t)(count > 0 ? count : 1));
    for (int i = 0; i < count; ++i) {
        out[i] = beats[i] * stepSizeFrames + stepSizeFrames / 2;
    }
    *beats_out = out;
    return count;
}

// Dominant key as a Mixxx key index: 1..12 = C..B major, 13..24 = C..B minor,
// 0 = none. (Mixxx keeps a key-change timeline and picks the global key; this
// takes the most frequently reported key across windows.)
int qm_key(const float* mono, int n, int sr) {
    if (!mono || n <= 0 || sr <= 0) {
        return 0;
    }
    GetKeyMode::Config config((double)sr, 440.0f);
    GetKeyMode keyMode(config);
    int blockSize = keyMode.getBlockSize();
    int hopSize = keyMode.getHopSize();
    if (blockSize <= 0 || hopSize <= 0) {
        return 0;
    }

    KeyCtx ctx;
    ctx.keyMode = &keyMode;
    for (int i = 0; i < 25; ++i) {
        ctx.counts[i] = 0;
    }
    overlapFeed(mono, (long)n, blockSize, hopSize, &ctx, keyCallback);

    int best = 0;
    int bestCount = 0;
    for (int k = 1; k <= 24; ++k) {
        if (ctx.counts[k] > bestCount) {
            bestCount = ctx.counts[k];
            best = k;
        }
    }
    return best;
}

void qm_free(void* p) {
    free(p);
}

}  // extern "C"
