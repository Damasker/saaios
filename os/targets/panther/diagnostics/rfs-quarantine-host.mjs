/*
 * Host-only, in-memory RFS transaction model. NEVER install on a phone or
 * connect its returned frames to a device. It has no filesystem, device,
 * network, or promotion API. The grant copies cmd6 seq=1; stock rfsd's
 * normal cmd2 data dispatch compares it with the incoming sequence. This
 * still omits factory filesystem state and is NOT device-ready.
 */
import { createHash, timingSafeEqual } from 'node:crypto';

export const HOST_ONLY_ACK = 'HOST_ONLY_QUARANTINE_NO_DEVICE_IO';
export const BASELINE_BYTES = 524288;
export const WRITE_BYTES = 189446;
export const CHUNK_BYTES = 2012;
export const MAX_GRANTS = 95;

const REQUEST_7 = Buffer.from('070000000400000003000000', 'hex');
const REQUEST_3 = Buffer.from('030000000c000000000000000300000000000000', 'hex');
const REQUEST_6 = Buffer.from('0600010010000000030000000000000006e4020002000000', 'hex');
const STATUS_7 = Buffer.from('03000000080000000000000003000000', 'hex');
const STATUS_COMPLETE = Buffer.from('03000100080000000000000003000000', 'hex');
const MAX_FRAME_BYTES = 8 + 12 + CHUNK_BYTES;
const OBSERVED_GRANT_SEQUENCE = 1;
const REQUIRED_DATA_SEQUENCE = OBSERVED_GRANT_SEQUENCE;

export class RfsQuarantineError extends Error {
  constructor(code) {
    super(code);
    this.name = 'RfsQuarantineError';
    this.code = code;
  }
}

function refuse(code) {
  throw new RfsQuarantineError(code);
}

function sha256(buffer) {
  return createHash('sha256').update(buffer).digest();
}

function parseFrame(frame) {
  if (!Buffer.isBuffer(frame) || frame.length < 8 ||
      frame.length > MAX_FRAME_BYTES) refuse('invalid_frame_size');
  const payloadLength = frame.readUInt32LE(4);
  if (payloadLength > MAX_FRAME_BYTES - 8 ||
      frame.length !== 8 + payloadLength) refuse('invalid_frame_length');
  return {
    command: frame.readUInt16LE(0),
    sequence: frame.readUInt16LE(2),
    payloadLength,
  };
}

function exact(frame, expected, code) {
  if (frame.length !== expected.length ||
      !timingSafeEqual(frame, expected)) refuse(code);
}

export class HostOnlyRfsTransaction {
  #baseline;
  #baselineDigest;
  #candidate = null;
  #state = 'WAIT_7';
  #offset = 0;
  #expectedChunk = 0;
  #grants = 0;
  #receivedHash = null;

  constructor({ acknowledge, baseline, baselineSha256 } = {}) {
    if (acknowledge !== HOST_ONLY_ACK) refuse('host_only_ack_required');
    if (!Buffer.isBuffer(baseline) || baseline.length !== BASELINE_BYTES)
      refuse('invalid_baseline_size');
    if (typeof baselineSha256 !== 'string' ||
        !/^[0-9a-fA-F]{64}$/.test(baselineSha256))
      refuse('invalid_pinned_sha256');
    const pinned = Buffer.from(baselineSha256, 'hex');
    this.#baseline = Buffer.from(baseline); // private, never returned or written
    this.#baselineDigest = sha256(this.#baseline);
    if (!timingSafeEqual(this.#baselineDigest, pinned)) {
      this.#baseline.fill(0);
      refuse('baseline_sha256_mismatch');
    }
  }

  get state() { return this.#state; }
  get grantsIssued() { return this.#grants; }
  get bytesReceived() { return this.#offset; }
  get candidatePrepared() { return this.#candidate !== null; }

  #abort() {
    if (this.#candidate) this.#candidate.fill(0);
    this.#candidate = null;
    this.#receivedHash = null;
    this.#state = 'ABORTED';
  }

  #prepareCandidate() {
    const clone = Buffer.from(this.#baseline);
    if (clone.length !== BASELINE_BYTES ||
        !timingSafeEqual(sha256(clone), this.#baselineDigest)) {
      clone.fill(0);
      refuse('candidate_clone_invalid');
    }
    this.#candidate = clone;
  }

  #grant() {
    if (this.#offset >= WRITE_BYTES || this.#grants >= MAX_GRANTS)
      refuse('grant_bound_exceeded');
    const chunk = Math.min(CHUNK_BYTES, WRITE_BYTES - this.#offset);
    const grant = Buffer.alloc(20);
    grant.writeUInt16LE(2, 0);
    grant.writeUInt16LE(OBSERVED_GRANT_SEQUENCE, 2);
    grant.writeUInt32LE(12, 4);
    grant.writeUInt32LE(3, 8);
    grant.writeUInt32LE(this.#offset, 12);
    grant.writeUInt32LE(chunk, 16);
    this.#expectedChunk = chunk;
    this.#grants++;
    return grant;
  }

  #acceptData(frame, header) {
    if (header.command !== 2 ||
        header.sequence !== REQUIRED_DATA_SEQUENCE ||
        header.payloadLength !== 12 + this.#expectedChunk ||
        frame.length !== 20 + this.#expectedChunk)
      refuse('invalid_data_header');
    const status = frame.readUInt32LE(8);
    const fileId = frame.readUInt32LE(12);
    const chunkLength = frame.readUInt32LE(16);
    if (status !== 0 || fileId !== 3 ||
        chunkLength !== this.#expectedChunk ||
        this.#offset + chunkLength > WRITE_BYTES)
      refuse('invalid_data_payload_header');
    this.#receivedHash.update(frame.subarray(20, 20 + chunkLength));
    if (frame.copy(this.#candidate, this.#offset, 20, 20 + chunkLength) !==
        chunkLength) refuse('short_candidate_copy');
    this.#offset += chunkLength;
    if (this.#offset === WRITE_BYTES) {
      if (this.#grants !== MAX_GRANTS ||
          this.#candidate.length !== BASELINE_BYTES ||
          !timingSafeEqual(sha256(this.#baseline), this.#baselineDigest) ||
          !timingSafeEqual(
            sha256(this.#candidate.subarray(0, WRITE_BYTES)),
            this.#receivedHash.digest()) ||
          !timingSafeEqual(
            this.#candidate.subarray(WRITE_BYTES),
            this.#baseline.subarray(WRITE_BYTES)))
        refuse('completion_invariant_failed');
      this.#receivedHash = null;
      this.#state = 'QUARANTINED_COMPLETE';
      // Factory sends this after fsync/OnWriteDone. This in-memory model
      // cannot prove durability and must never be connected to a device.
      return Buffer.from(STATUS_COMPLETE);
    }
    return this.#grant();
  }

  /* Returns an isolated host Buffer containing the next modeled response,
   * or null. Any malformed/unexpected input permanently aborts the session. */
  acceptFrame(frame) {
    if (this.#state === 'ABORTED') refuse('session_aborted');
    if (this.#state === 'QUARANTINED_COMPLETE') {
      this.#abort();
      refuse('frame_after_completion');
    }
    try {
      const header = parseFrame(frame);
      if (this.#state === 'WAIT_7') {
        exact(frame, REQUEST_7, 'unexpected_cmd7');
        // Preparation and digest verification precede the status-0 reply.
        this.#prepareCandidate();
        this.#state = 'WAIT_3';
        return Buffer.from(STATUS_7);
      }
      if (this.#state === 'WAIT_3') {
        exact(frame, REQUEST_3, 'unexpected_cmd3');
        this.#state = 'WAIT_6';
        return null;
      }
      if (this.#state === 'WAIT_6') {
        exact(frame, REQUEST_6, 'unexpected_cmd6');
        if (!this.#candidate) refuse('candidate_not_prepared');
        this.#receivedHash = createHash('sha256');
        this.#state = 'WAIT_DATA';
        return this.#grant();
      }
      if (this.#state === 'WAIT_DATA') return this.#acceptData(frame, header);
      refuse('invalid_state');
    } catch (error) {
      this.#abort();
      if (error instanceof RfsQuarantineError) throw error;
      refuse('internal_error');
    }
  }

  /* Read-only copy for host fixtures/review. There is intentionally no
   * filesystem output or candidate promotion method. */
  quarantinedSnapshot() {
    if (this.#state !== 'QUARANTINED_COMPLETE')
      refuse('candidate_not_complete');
    return Buffer.from(this.#candidate);
  }
}
