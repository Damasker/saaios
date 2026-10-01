import test from 'node:test';
import assert from 'node:assert/strict';
import {
  BASELINE_BYTES, CHUNK_BYTES, HOST_ONLY_ACK, HostOnlyRfsTransaction,
  MAX_GRANTS, RfsQuarantineError, WRITE_BYTES,
} from './rfs-quarantine-host.mjs';

/* Independently pinned synthetic 512 KiB baseline, byte 0x5a throughout. */
const SYNTHETIC_SHA256 =
  '0d57ce7e6f299b77f1aa75b8b0198aaaa910b6fd2ea09bfc4a5fcc4d2023f5d2';
const CMD7 = Buffer.from('070000000400000003000000', 'hex');
const CMD3 = Buffer.from('030000000c000000000000000300000000000000', 'hex');
const CMD6 = Buffer.from('0600010010000000030000000000000006e4020002000000', 'hex');
const STATUS7 = Buffer.from('03000000080000000000000003000000', 'hex');
const STATUS_COMPLETE = Buffer.from('03000100080000000000000003000000', 'hex');

function transaction(baseline = Buffer.alloc(BASELINE_BYTES, 0x5a)) {
  return new HostOnlyRfsTransaction({
    acknowledge: HOST_ONLY_ACK,
    baseline,
    baselineSha256: SYNTHETIC_SHA256,
  });
}

function throughGrant(tx) {
  assert.equal(tx.candidatePrepared, false);
  assert.throws(() => tx.quarantinedSnapshot(),
    { code: 'candidate_not_complete' });
  assert.deepEqual(tx.acceptFrame(CMD7), STATUS7);
  assert.equal(tx.candidatePrepared, true); // Prepared before status-0 cmd7 reply.
  assert.throws(() => tx.quarantinedSnapshot(),
    { code: 'candidate_not_complete' });
  assert.equal(tx.acceptFrame(CMD3), null);
  assert.throws(() => tx.quarantinedSnapshot(),
    { code: 'candidate_not_complete' });
  return tx.acceptFrame(CMD6);
}

function dataFrame(grant, fill = 0x11) {
  const chunk = grant.readUInt32LE(16);
  const frame = Buffer.alloc(20 + chunk, fill);
  frame.writeUInt16LE(2, 0);
  frame.writeUInt16LE(grant.readUInt16LE(2), 2);
  frame.writeUInt32LE(12 + chunk, 4);
  frame.writeUInt32LE(0, 8);
  frame.writeUInt32LE(3, 12);
  frame.writeUInt32LE(chunk, 16);
  return frame;
}

function complete(tx, fillForStep = () => 0x11) {
  let grant = throughGrant(tx);
  const expected = Buffer.alloc(BASELINE_BYTES, 0x5a);
  for (let step = 0; step < MAX_GRANTS; step++) {
    assert.equal(grant.length, 20);
    assert.equal(grant.readUInt16LE(0), 2);
    assert.equal(grant.readUInt16LE(2), 1);
    assert.equal(grant.readUInt32LE(4), 12);
    assert.equal(grant.readUInt32LE(8), 3);
    const offset = grant.readUInt32LE(12);
    const chunk = grant.readUInt32LE(16);
    assert.equal(offset, step * CHUNK_BYTES);
    assert.equal(chunk, step === MAX_GRANTS - 1 ? 318 : CHUNK_BYTES);
    const fill = fillForStep(step);
    expected.fill(fill, offset, offset + chunk);
    const answer = tx.acceptFrame(dataFrame(grant, fill));
    if (step < MAX_GRANTS - 1) {
      assert.ok(answer);
      grant = answer;
      assert.throws(() => tx.quarantinedSnapshot(),
        { code: 'candidate_not_complete' });
    } else {
      assert.deepEqual(answer, STATUS_COMPLETE);
    }
  }
  assert.equal(tx.grantsIssued, MAX_GRANTS);
  assert.equal(tx.bytesReceived, WRITE_BYTES);
  assert.equal(tx.state, 'QUARANTINED_COMPLETE');
  assert.deepEqual(tx.quarantinedSnapshot(), expected);
  return expected;
}

test('95 bounded grants overwrite only private prefix, then return exact final ACK', () => {
  const baseline = Buffer.alloc(BASELINE_BYTES, 0x5a);
  const tx = transaction(baseline);
  const expected = complete(tx, (step) => step & 0xff);
  assert.ok(baseline.equals(Buffer.alloc(BASELINE_BYTES, 0x5a)));
  assert.ok(expected.subarray(WRITE_BYTES).equals(
    Buffer.alloc(BASELINE_BYTES - WRITE_BYTES, 0x5a)));
  const detached = tx.quarantinedSnapshot();
  detached[0] ^= 0xff;
  assert.notEqual(detached[0], tx.quarantinedSnapshot()[0]);
});

test('host opt-in and out-of-band baseline pin are required', () => {
  const baseline = Buffer.alloc(BASELINE_BYTES, 0x5a);
  assert.throws(() => new HostOnlyRfsTransaction({
    baseline, baselineSha256: SYNTHETIC_SHA256,
  }), { code: 'host_only_ack_required' });
  assert.throws(() => transaction(Buffer.alloc(BASELINE_BYTES - 1, 0x5a)),
    { code: 'invalid_baseline_size' });
  const changed = Buffer.from(baseline);
  changed[0] ^= 1;
  assert.throws(() => transaction(changed),
    { code: 'baseline_sha256_mismatch' });
  const tx = transaction(baseline);
  baseline.fill(0); // Caller mutation cannot alter the private pinned copy.
  complete(tx);
});

test('malformed or out-of-order cmd7, cmd3 and cmd6 abort without candidate', () => {
  for (const [phase, original] of [
    ['WAIT_7', CMD7], ['WAIT_3', CMD3], ['WAIT_6', CMD6],
  ]) {
    for (const mutated of [
      original.subarray(0, original.length - 1),
      (() => { const b = Buffer.from(original); b[b.length - 1] ^= 1; return b; })(),
    ]) {
      const tx = transaction();
      if (phase !== 'WAIT_7') tx.acceptFrame(CMD7);
      if (phase === 'WAIT_6') tx.acceptFrame(CMD3);
      assert.throws(() => tx.acceptFrame(mutated), RfsQuarantineError);
      assert.equal(tx.state, 'ABORTED');
      assert.throws(() => tx.quarantinedSnapshot(),
        { code: 'candidate_not_complete' });
    }
  }
  const tx = transaction();
  assert.throws(() => tx.acceptFrame(CMD3),
    { code: 'unexpected_cmd7' });
});

test('malformed cmd2 data fails closed: seq, status, id, length, body, size', () => {
  const cases = [
    (b) => b.writeUInt16LE(2, 2),
    (b) => b.writeUInt32LE(1, 8),
    (b) => b.writeUInt32LE(4, 12),
    (b) => b.writeUInt32LE(CHUNK_BYTES - 1, 16),
    (b) => b.writeUInt32LE(12 + CHUNK_BYTES - 1, 4),
    (b) => b.subarray(0, b.length - 1),
    (b) => Buffer.concat([b, Buffer.alloc(1)]),
  ];
  for (const mutate of cases) {
    const tx = transaction();
    const grant = throughGrant(tx);
    const frame = dataFrame(grant);
    const changed = mutate(frame);
    assert.throws(() => tx.acceptFrame(Buffer.isBuffer(changed) ? changed : frame),
      RfsQuarantineError);
    assert.equal(tx.state, 'ABORTED');
    assert.throws(() => tx.quarantinedSnapshot(),
      { code: 'candidate_not_complete' });
  }
});

test('partial transfer aborts, and extra frames after completion cannot promote', () => {
  const tx = transaction();
  const firstGrant = throughGrant(tx);
  const secondGrant = tx.acceptFrame(dataFrame(firstGrant));
  assert.equal(tx.bytesReceived, CHUNK_BYTES);
  const bad = dataFrame(secondGrant);
  bad.writeUInt32LE(1, 8);
  assert.throws(() => tx.acceptFrame(bad), RfsQuarantineError);
  assert.equal(tx.state, 'ABORTED');
  assert.throws(() => tx.acceptFrame(dataFrame(secondGrant)),
    { code: 'session_aborted' });

  const completed = transaction();
  complete(completed);
  assert.throws(() => completed.acceptFrame(CMD7),
    { code: 'frame_after_completion' });
  assert.equal(completed.state, 'ABORTED');
  assert.throws(() => completed.quarantinedSnapshot(),
    { code: 'candidate_not_complete' });
});

test('a bad final chunk aborts without final ACK or candidate exposure', () => {
  const tx = transaction();
  let grant = throughGrant(tx);
  for (let step = 0; step < MAX_GRANTS - 1; step++) {
    grant = tx.acceptFrame(dataFrame(grant, step & 0xff));
  }
  assert.equal(tx.grantsIssued, MAX_GRANTS);
  assert.equal(tx.bytesReceived, WRITE_BYTES - 318);
  const final = dataFrame(grant);
  final.writeUInt32LE(1, 8); // CP-reported failure, never acknowledge success.
  assert.throws(() => tx.acceptFrame(final),
    { code: 'invalid_data_payload_header' });
  assert.equal(tx.state, 'ABORTED');
  assert.equal(tx.candidatePrepared, false);
  assert.throws(() => tx.quarantinedSnapshot(),
    { code: 'candidate_not_complete' });
});

test('each transaction gets a fresh candidate clone', () => {
  const one = transaction();
  const two = transaction();
  complete(one, () => 0x11);
  complete(two, () => 0x22);
  assert.notDeepEqual(one.quarantinedSnapshot(), two.quarantinedSnapshot());
});
