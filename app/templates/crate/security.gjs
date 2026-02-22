import { htmlSafe } from '@ember/template';

import CrateHeader from 'crates-io/components/crate-header';

// CVSS 3.0/3.1 metric weights
const CVSS3_WEIGHTS = {
  AV: { N: 0.85, A: 0.62, L: 0.55, P: 0.2 },
  AC: { L: 0.77, H: 0.44 },
  // PR values depend on Scope - [Unchanged, Changed]
  PR: { N: [0.85, 0.85], L: [0.62, 0.68], H: [0.27, 0.5] },
  UI: { N: 0.85, R: 0.62 },
  C: { H: 0.56, L: 0.22, N: 0 },
  I: { H: 0.56, L: 0.22, N: 0 },
  A: { H: 0.56, L: 0.22, N: 0 },
};

// CVSS 4.0 MacroVector lookup table (EQ1-EQ6 combinations → base score)
// Based on CVSS 4.0 specification from FIRST.org
// prettier-ignore
const CVSS4_LOOKUP = {
  '000000': 10, '000001': 9.9, '000010': 9.8, '000011': 9.5, '000020': 9.5, '000021': 9.2,
  '000100': 10, '000101': 9.6, '000110': 9.3, '000111': 8.7, '000120': 9.1, '000121': 8.1,
  '000200': 9.3, '000201': 9, '000210': 8.9, '000211': 8, '000220': 8.1, '000221': 6.8,
  '001000': 9.8, '001001': 9.5, '001010': 9.5, '001011': 9.2, '001020': 9, '001021': 8.4,
  '001100': 9.3, '001101': 9.2, '001110': 8.9, '001111': 8.1, '001120': 8.1, '001121': 6.5,
  '001200': 8.8, '001201': 8, '001210': 7.8, '001211': 7, '001220': 6.9, '001221': 4.8,
  '002001': 9.2, '002011': 8.2, '002021': 7.2, '002101': 7.9, '002111': 6.9, '002121': 5,
  '002201': 6.9, '002211': 5.5, '002221': 2.7,
  '010000': 9.9, '010001': 9.7, '010010': 9.5, '010011': 9.2, '010020': 9.2, '010021': 8.5,
  '010100': 9.5, '010101': 9.1, '010110': 9, '010111': 8.3, '010120': 8.4, '010121': 7.1,
  '010200': 9.2, '010201': 8.1, '010210': 8.2, '010211': 7.1, '010220': 7.2, '010221': 5.3,
  '011000': 9.5, '011001': 9.3, '011010': 9.2, '011011': 8.5, '011020': 8.5, '011021': 7.3,
  '011100': 9.2, '011101': 8.2, '011110': 8, '011111': 7.2, '011120': 7, '011121': 5.9,
  '011200': 8.4, '011201': 7, '011210': 7.1, '011211': 5.2, '011220': 5, '011221': 3,
  '012001': 8.6, '012011': 7.5, '012021': 5.2, '012101': 7.1, '012111': 5.2, '012121': 2.9,
  '012201': 6.3, '012211': 2.9, '012221': 1.7,
  '100000': 9.8, '100001': 9.5, '100010': 9.4, '100011': 8.7, '100020': 9.1, '100021': 8.1,
  '100100': 9.4, '100101': 8.9, '100110': 8.6, '100111': 7.4, '100120': 7.7, '100121': 6.4,
  '100200': 8.7, '100201': 7.5, '100210': 7.4, '100211': 6.3, '100220': 6.3, '100221': 4.9,
  '101000': 9.4, '101001': 8.9, '101010': 8.8, '101011': 7.7, '101020': 7.6, '101021': 6.7,
  '101100': 8.6, '101101': 7.6, '101110': 7.4, '101111': 5.8, '101120': 5.9, '101121': 5,
  '101200': 7.2, '101201': 5.7, '101210': 5.7, '101211': 5.2, '101220': 5.2, '101221': 2.5,
  '102001': 8.3, '102011': 7, '102021': 5.4, '102101': 6.5, '102111': 5.8, '102121': 2.6,
  '102201': 5.3, '102211': 2.1, '102221': 1.3,
  '110000': 9.5, '110001': 9, '110010': 8.8, '110011': 7.6, '110020': 7.6, '110021': 7,
  '110100': 9, '110101': 7.7, '110110': 7.5, '110111': 6.2, '110120': 6.1, '110121': 5.3,
  '110200': 7.7, '110201': 6.6, '110210': 6.8, '110211': 5.9, '110220': 5.2, '110221': 3,
  '111000': 8.9, '111001': 7.8, '111010': 7.6, '111011': 6.7, '111020': 6.2, '111021': 5.8,
  '111100': 7.4, '111101': 5.9, '111110': 5.7, '111111': 5.7, '111120': 4.7, '111121': 2.3,
  '111200': 6.1, '111201': 5.2, '111210': 5.7, '111211': 2.9, '111220': 2.4, '111221': 1.6,
  '112001': 7.1, '112011': 5.9, '112021': 3, '112101': 5.8, '112111': 2.6, '112121': 1.5,
  '112201': 2.3, '112211': 1.3, '112221': 0.6,
  '200000': 9.3, '200001': 8.7, '200010': 8.6, '200011': 7.2, '200020': 7.5, '200021': 5.8,
  '200100': 8.6, '200101': 7.4, '200110': 7.4, '200111': 6.1, '200120': 5.6, '200121': 3.4,
  '200200': 7, '200201': 5.4, '200210': 5.2, '200211': 4, '200220': 4, '200221': 2.2,
  '201000': 8.5, '201001': 7.5, '201010': 7.4, '201011': 5.5, '201020': 6.2, '201021': 5.1,
  '201100': 7.2, '201101': 5.7, '201110': 5.5, '201111': 4.1, '201120': 4.6, '201121': 1.9,
  '201200': 5.3, '201201': 3.6, '201210': 3.4, '201211': 1.9, '201220': 1.9, '201221': 0.8,
  '202001': 6.4, '202011': 5.1, '202021': 2, '202101': 4.7, '202111': 2.1, '202121': 1.1,
  '202201': 2.4, '202211': 0.9, '202221': 0.4,
  '210000': 8.8, '210001': 7.5, '210010': 7.3, '210011': 5.3, '210020': 6, '210021': 5,
  '210100': 7.3, '210101': 5.5, '210110': 5.9, '210111': 4, '210120': 4.1, '210121': 2,
  '210200': 5.4, '210201': 4.3, '210210': 4.5, '210211': 2.2, '210220': 2, '210221': 1.1,
  '211000': 7.5, '211001': 5.5, '211010': 5.8, '211011': 4.5, '211020': 4, '211021': 2.1,
  '211100': 6.1, '211101': 5.1, '211110': 4.8, '211111': 1.8, '211120': 2, '211121': 0.9,
  '211200': 4.6, '211201': 1.8, '211210': 1.7, '211211': 0.7, '211220': 0.8, '211221': 0.2,
  '212001': 5.3, '212011': 2.4, '212021': 1.4, '212101': 2.4, '212111': 1.2, '212121': 0.5,
  '212201': 1, '212211': 0.3, '212221': 0.1,
};

// CVSS 4.0 metric value mappings for EQ calculation
const CVSS4_METRICS = {
  AV: { N: 0, A: 1, L: 2, P: 3 },
  AC: { L: 0, H: 1 },
  AT: { N: 0, P: 1 },
  PR: { N: 0, L: 1, H: 2 },
  UI: { N: 0, P: 1, A: 2 },
  VC: { H: 0, L: 1, N: 2 },
  VI: { H: 0, L: 1, N: 2 },
  VA: { H: 0, L: 1, N: 2 },
  SC: { H: 0, L: 1, N: 2 },
  SI: { S: 0, H: 1, L: 2, N: 3 },
  SA: { S: 0, H: 1, L: 2, N: 3 },
};

// CVSS 4.0 severity distance values for interpolation (from FIRST.org calculator)
// These represent distance from "highest severity" within each metric
const CVSS4_DISTANCES = {
  AV: { N: 0, A: 0.1, L: 0.2, P: 0.3 },
  PR: { N: 0, L: 0.1, H: 0.2 },
  UI: { N: 0, P: 0.1, A: 0.2 },
  AC: { L: 0, H: 0.1 },
  AT: { N: 0, P: 0.1 },
  VC: { H: 0, L: 0.1, N: 0.2 },
  VI: { H: 0, L: 0.1, N: 0.2 },
  VA: { H: 0, L: 0.1, N: 0.2 },
  SC: { H: 0.1, L: 0.2, N: 0.3 },
  SI: { S: 0, H: 0.1, L: 0.2, N: 0.3 },
  SA: { S: 0, H: 0.1, L: 0.2, N: 0.3 },
  CR: { H: 0, M: 0.1, L: 0.2 },
  IR: { H: 0, M: 0.1, L: 0.2 },
  AR: { H: 0, M: 0.1, L: 0.2 },
};

// Maximum severity depths for each EQ level (from FIRST.org calculator)
const CVSS4_MAX_SEVERITY = {
  eq1: { 0: 1, 1: 4, 2: 5 },
  eq2: { 0: 1, 1: 2 },
  eq3eq6: { '00': 7, '01': 6, '10': 8, '11': 8, '21': 10 },
  eq4: { 0: 6, 1: 5, 2: 4 },
};

// Highest severity vectors for each EQ level (from FIRST.org calculator)
// prettier-ignore
const CVSS4_MAX_COMPOSED = {
  eq1: {
    0: [{ AV: 'N', PR: 'N', UI: 'N' }],
    1: [{ AV: 'A', PR: 'N', UI: 'N' }, { AV: 'N', PR: 'L', UI: 'N' }, { AV: 'N', PR: 'N', UI: 'P' }],
    2: [{ AV: 'P', PR: 'N', UI: 'N' }, { AV: 'A', PR: 'L', UI: 'P' }],
  },
  eq2: {
    0: [{ AC: 'L', AT: 'N' }],
    1: [{ AC: 'H', AT: 'N' }, { AC: 'L', AT: 'P' }],
  },
  eq3eq6: {
    '00': [{ VC: 'H', VI: 'H', VA: 'H', CR: 'H', IR: 'H', AR: 'H' }],
    '01': [{ VC: 'H', VI: 'H', VA: 'L', CR: 'M', IR: 'M', AR: 'H' }, { VC: 'H', VI: 'H', VA: 'H', CR: 'M', IR: 'M', AR: 'M' }],
    '10': [{ VC: 'L', VI: 'H', VA: 'H', CR: 'H', IR: 'H', AR: 'H' }, { VC: 'H', VI: 'L', VA: 'H', CR: 'H', IR: 'H', AR: 'H' }],
    '11': [{ VC: 'L', VI: 'H', VA: 'L', CR: 'H', IR: 'M', AR: 'H' }, { VC: 'L', VI: 'H', VA: 'H', CR: 'H', IR: 'M', AR: 'M' }, { VC: 'H', VI: 'L', VA: 'H', CR: 'M', IR: 'H', AR: 'M' }, { VC: 'H', VI: 'L', VA: 'L', CR: 'M', IR: 'H', AR: 'H' }, { VC: 'L', VI: 'L', VA: 'H', CR: 'H', IR: 'H', AR: 'M' }],
    '21': [{ VC: 'L', VI: 'L', VA: 'L', CR: 'H', IR: 'H', AR: 'H' }],
  },
  eq4: {
    0: [{ SC: 'H', SI: 'S', SA: 'S' }],
    1: [{ SC: 'H', SI: 'H', SA: 'H' }],
    2: [{ SC: 'L', SI: 'L', SA: 'L' }],
  },
};

function roundUp(value) {
  // CVSS "roundup" function: round up to nearest 0.1
  return Math.ceil(value * 10) / 10;
}

function calculateCvss3Score(metrics) {
  let scopeChanged = metrics.S === 'C';
  let av = CVSS3_WEIGHTS.AV[metrics.AV];
  let ac = CVSS3_WEIGHTS.AC[metrics.AC];
  let pr = CVSS3_WEIGHTS.PR[metrics.PR]?.[scopeChanged ? 1 : 0];
  let ui = CVSS3_WEIGHTS.UI[metrics.UI];
  let c = CVSS3_WEIGHTS.C[metrics.C];
  let i = CVSS3_WEIGHTS.I[metrics.I];
  let a = CVSS3_WEIGHTS.A[metrics.A];

  if ([av, ac, pr, ui, c, i, a].includes(undefined)) {
    return null;
  }

  // Impact Sub-Score (ISS)
  let isc_base = 1 - (1 - c) * (1 - i) * (1 - a);

  // Impact
  let impact = scopeChanged ? 7.52 * (isc_base - 0.029) - 3.25 * Math.pow(isc_base - 0.02, 15) : 6.42 * isc_base;

  // Exploitability
  let exploitability = 8.22 * av * ac * pr * ui;

  // Base Score
  if (impact <= 0) {
    return 0;
  }

  // prettier-ignore
  return scopeChanged ? roundUp(Math.min(1.08 * (impact + exploitability), 10)) : roundUp(Math.min(impact + exploitability, 10));
}

function calculateCvss4Score(metrics) {
  // Get metric values with defaults
  let av = CVSS4_METRICS.AV[metrics.AV] ?? 0;
  let ac = CVSS4_METRICS.AC[metrics.AC] ?? 0;
  let at = CVSS4_METRICS.AT[metrics.AT] ?? 0;
  let pr = CVSS4_METRICS.PR[metrics.PR] ?? 0;
  let ui = CVSS4_METRICS.UI[metrics.UI] ?? 0;
  let vc = CVSS4_METRICS.VC[metrics.VC] ?? 0;
  let vi = CVSS4_METRICS.VI[metrics.VI] ?? 0;
  let va = CVSS4_METRICS.VA[metrics.VA] ?? 0;
  let sc = CVSS4_METRICS.SC[metrics.SC] ?? 3;
  let si = CVSS4_METRICS.SI[metrics.SI] ?? 3;
  let sa = CVSS4_METRICS.SA[metrics.SA] ?? 3;

  // Get effective metric values (with defaults for environmental)
  let mAV = metrics.AV ?? 'N';
  let mPR = metrics.PR ?? 'N';
  let mUI = metrics.UI ?? 'N';
  let mAC = metrics.AC ?? 'L';
  let mAT = metrics.AT ?? 'N';
  let mVC = metrics.VC ?? 'H';
  let mVI = metrics.VI ?? 'H';
  let mVA = metrics.VA ?? 'H';
  let mSC = metrics.SC ?? 'N';
  let mSI = metrics.SI ?? 'N';
  let mSA = metrics.SA ?? 'N';
  let mCR = metrics.CR ?? 'H';
  let mIR = metrics.IR ?? 'H';
  let mAR = metrics.AR ?? 'H';

  // Compute equivalence classes (EQ1-EQ6)
  // EQ1: 0 = AV:N AND PR:N AND UI:N
  //      1 = (AV:N OR PR:N OR UI:N) AND NOT(AV:N AND PR:N AND UI:N) AND NOT AV:P
  //      2 = NOT(AV:N OR PR:N OR UI:N) OR AV:P
  let eq1;
  if (av === 0 && pr === 0 && ui === 0) {
    eq1 = 0;
  } else if ((av === 0 || pr === 0 || ui === 0) && av !== 3) {
    eq1 = 1;
  } else {
    eq1 = 2;
  }

  // EQ2: 0 = AC:L AND AT:N, 1 = otherwise
  let eq2 = ac === 0 && at === 0 ? 0 : 1;

  // EQ3: 0 = VC:H AND VI:H
  //      1 = NOT(VC:H AND VI:H) AND (VC:H OR VI:H OR VA:H)
  //      2 = NOT(VC:H OR VI:H OR VA:H)
  let eq3;
  if (vc === 0 && vi === 0) {
    eq3 = 0;
  } else if (vc === 0 || vi === 0 || va === 0) {
    eq3 = 1;
  } else {
    eq3 = 2;
  }

  // EQ4: 0 = MSI:S OR MSA:S
  //      1 = NOT(MSI:S OR MSA:S) AND (SC:H OR SI:H OR SA:H)
  //      2 = NOT(MSI:S OR MSA:S) AND NOT(SC:H OR SI:H OR SA:H)
  let eq4;
  if (si === 0 || sa === 0) {
    eq4 = 0;
  } else if (sc === 0 || si === 1 || sa === 1) {
    eq4 = 1;
  } else {
    eq4 = 2;
  }

  // EQ5: 0 = E:A, 1 = E:P, 2 = E:U
  let eq5 = metrics.E === 'U' ? 2 : metrics.E === 'P' ? 1 : 0;

  // EQ6: 0 = (CR:H AND VC:H) OR (IR:H AND VI:H) OR (AR:H AND VA:H), 1 = NOT the above
  // prettier-ignore
  let eq6 = (mCR === 'H' && mVC === 'H') || (mIR === 'H' && mVI === 'H') || (mAR === 'H' && mVA === 'H') ? 0 : 1;

  // Step 1: Look up base score from MacroVector
  let macroVector = `${eq1}${eq2}${eq3}${eq4}${eq5}${eq6}`;
  let baseScore = CVSS4_LOOKUP[macroVector];
  if (baseScore === undefined) return null;

  // Exception for no impact on system
  if (mVC === 'N' && mVI === 'N' && mVA === 'N' && mSC === 'N' && mSI === 'N' && mSA === 'N') {
    return 0;
  }

  // Step 2: Find next lower MacroVector scores for each EQ
  let scoreEq1NextLower = CVSS4_LOOKUP[`${eq1 + 1}${eq2}${eq3}${eq4}${eq5}${eq6}`];
  let scoreEq2NextLower = CVSS4_LOOKUP[`${eq1}${eq2 + 1}${eq3}${eq4}${eq5}${eq6}`];
  let scoreEq4NextLower = CVSS4_LOOKUP[`${eq1}${eq2}${eq3}${eq4 + 1}${eq5}${eq6}`];
  let scoreEq5NextLower = CVSS4_LOOKUP[`${eq1}${eq2}${eq3}${eq4}${eq5 + 1}${eq6}`];

  // EQ3+EQ6 are related - determine next lower based on current state
  let scoreEq3eq6NextLower;
  if (eq3 === 1 && eq6 === 1) {
    scoreEq3eq6NextLower = CVSS4_LOOKUP[`${eq1}${eq2}${eq3 + 1}${eq4}${eq5}${eq6}`];
  } else if (eq3 === 0 && eq6 === 1) {
    scoreEq3eq6NextLower = CVSS4_LOOKUP[`${eq1}${eq2}${eq3 + 1}${eq4}${eq5}${eq6}`];
  } else if (eq3 === 1 && eq6 === 0) {
    scoreEq3eq6NextLower = CVSS4_LOOKUP[`${eq1}${eq2}${eq3}${eq4}${eq5}${eq6 + 1}`];
  } else if (eq3 === 0 && eq6 === 0) {
    let left = CVSS4_LOOKUP[`${eq1}${eq2}${eq3}${eq4}${eq5}${eq6 + 1}`];
    let right = CVSS4_LOOKUP[`${eq1}${eq2}${eq3 + 1}${eq4}${eq5}${eq6}`];
    scoreEq3eq6NextLower = Math.max(left, right);
  } else {
    scoreEq3eq6NextLower = CVSS4_LOOKUP[`${eq1}${eq2}${eq3 + 1}${eq4}${eq5}${eq6}`];
  }

  // Step 3: Compose all max vectors from all EQ levels and find valid one
  let eq1Maxes = CVSS4_MAX_COMPOSED.eq1[eq1] || [];
  let eq2Maxes = CVSS4_MAX_COMPOSED.eq2[eq2] || [];
  let eq3eq6Key = `${eq3}${eq6}`;
  let eq3eq6Maxes = CVSS4_MAX_COMPOSED.eq3eq6[eq3eq6Key] || [];
  let eq4Maxes = CVSS4_MAX_COMPOSED.eq4[eq4] || [];

  // Calculate current severity distances
  let distAV = CVSS4_DISTANCES.AV[mAV] ?? 0;
  let distPR = CVSS4_DISTANCES.PR[mPR] ?? 0;
  let distUI = CVSS4_DISTANCES.UI[mUI] ?? 0;
  let distAC = CVSS4_DISTANCES.AC[mAC] ?? 0;
  let distAT = CVSS4_DISTANCES.AT[mAT] ?? 0;
  let distVC = CVSS4_DISTANCES.VC[mVC] ?? 0;
  let distVI = CVSS4_DISTANCES.VI[mVI] ?? 0;
  let distVA = CVSS4_DISTANCES.VA[mVA] ?? 0;
  let distSC = CVSS4_DISTANCES.SC[mSC] ?? 0;
  let distSI = CVSS4_DISTANCES.SI[mSI] ?? 0;
  let distSA = CVSS4_DISTANCES.SA[mSA] ?? 0;
  let distCR = CVSS4_DISTANCES.CR[mCR] ?? 0;
  let distIR = CVSS4_DISTANCES.IR[mIR] ?? 0;
  let distAR = CVSS4_DISTANCES.AR[mAR] ?? 0;

  // Find the first valid max vector combination (all distances >= 0)
  let sevDistEq1 = 0,
    sevDistEq2 = 0,
    sevDistEq3eq6 = 0,
    sevDistEq4 = 0;

  outer: for (let eq1Max of eq1Maxes) {
    for (let eq2Max of eq2Maxes) {
      for (let eq3eq6Max of eq3eq6Maxes) {
        for (let eq4Max of eq4Maxes) {
          // Calculate distances from this max vector
          let dAV = distAV - (CVSS4_DISTANCES.AV[eq1Max.AV] ?? 0);
          let dPR = distPR - (CVSS4_DISTANCES.PR[eq1Max.PR] ?? 0);
          let dUI = distUI - (CVSS4_DISTANCES.UI[eq1Max.UI] ?? 0);
          let dAC = distAC - (CVSS4_DISTANCES.AC[eq2Max.AC] ?? 0);
          let dAT = distAT - (CVSS4_DISTANCES.AT[eq2Max.AT] ?? 0);
          let dVC = distVC - (CVSS4_DISTANCES.VC[eq3eq6Max.VC] ?? 0);
          let dVI = distVI - (CVSS4_DISTANCES.VI[eq3eq6Max.VI] ?? 0);
          let dVA = distVA - (CVSS4_DISTANCES.VA[eq3eq6Max.VA] ?? 0);
          let dCR = distCR - (CVSS4_DISTANCES.CR[eq3eq6Max.CR] ?? 0);
          let dIR = distIR - (CVSS4_DISTANCES.IR[eq3eq6Max.IR] ?? 0);
          let dAR = distAR - (CVSS4_DISTANCES.AR[eq3eq6Max.AR] ?? 0);
          let dSC = distSC - (CVSS4_DISTANCES.SC[eq4Max.SC] ?? 0);
          let dSI = distSI - (CVSS4_DISTANCES.SI[eq4Max.SI] ?? 0);
          let dSA = distSA - (CVSS4_DISTANCES.SA[eq4Max.SA] ?? 0);

          // Check if all distances are non-negative
          if (
            dAV >= 0 &&
            dPR >= 0 &&
            dUI >= 0 &&
            dAC >= 0 &&
            dAT >= 0 &&
            dVC >= 0 &&
            dVI >= 0 &&
            dVA >= 0 &&
            dCR >= 0 &&
            dIR >= 0 &&
            dAR >= 0 &&
            dSC >= 0 &&
            dSI >= 0 &&
            dSA >= 0
          ) {
            sevDistEq1 = dAV + dPR + dUI;
            sevDistEq2 = dAC + dAT;
            sevDistEq3eq6 = dVC + dVI + dVA + dCR + dIR + dAR;
            sevDistEq4 = dSC + dSI + dSA;
            break outer;
          }
        }
      }
    }
  }

  // Step 4: Calculate normalized severities and mean distance
  let step = 0.1;
  let nExistingLower = 0;

  let maxSevEq1 = (CVSS4_MAX_SEVERITY.eq1[eq1] ?? 0) * step;
  let maxSevEq2 = (CVSS4_MAX_SEVERITY.eq2[eq2] ?? 0) * step;
  let maxSevEq3eq6 = (CVSS4_MAX_SEVERITY.eq3eq6[eq3eq6Key] ?? 0) * step;
  let maxSevEq4 = (CVSS4_MAX_SEVERITY.eq4[eq4] ?? 0) * step;

  let normSevEq1 = 0;
  if (scoreEq1NextLower !== undefined) {
    nExistingLower++;
    normSevEq1 = (sevDistEq1 / maxSevEq1) * (baseScore - scoreEq1NextLower);
  }

  let normSevEq2 = 0;
  if (scoreEq2NextLower !== undefined) {
    nExistingLower++;
    normSevEq2 = (sevDistEq2 / maxSevEq2) * (baseScore - scoreEq2NextLower);
  }

  let normSevEq3eq6 = 0;
  if (scoreEq3eq6NextLower !== undefined) {
    nExistingLower++;
    normSevEq3eq6 = (sevDistEq3eq6 / maxSevEq3eq6) * (baseScore - scoreEq3eq6NextLower);
  }

  let normSevEq4 = 0;
  if (scoreEq4NextLower !== undefined) {
    nExistingLower++;
    normSevEq4 = (sevDistEq4 / maxSevEq4) * (baseScore - scoreEq4NextLower);
  }

  let normSevEq5 = 0;
  if (scoreEq5NextLower !== undefined) {
    nExistingLower++;
    // For EQ5, percentage is always 0
    normSevEq5 = 0;
  }

  let meanDistance =
    nExistingLower === 0 ? 0 : (normSevEq1 + normSevEq2 + normSevEq3eq6 + normSevEq4 + normSevEq5) / nExistingLower;

  let score = baseScore - meanDistance;
  score = Math.max(0, Math.min(10, score));
  return Math.round(score * 10) / 10;
}

function calculateCvssScore(cvss) {
  let match = cvss.match(/^CVSS:(\d+\.\d+)\/(.+)$/);
  if (!match) return null;

  let version = match[1];
  let metrics = {};
  for (let part of match[2].split('/')) {
    let [key, value] = part.split(':');
    metrics[key] = value;
  }

  if (version === '3.0' || version === '3.1') {
    return calculateCvss3Score(metrics);
  } else if (version === '4.0') {
    return calculateCvss4Score(metrics);
  }
  return null;
}

function severityRating(score) {
  if (score === 0) return 'None';
  if (score < 4) return 'Low';
  if (score < 7) return 'Medium';
  if (score < 9) return 'High';
  return 'Critical';
}

function severityClass(score) {
  return `severity-${severityRating(score).toLowerCase()}`;
}

function aliasUrl(alias) {
  if (alias.startsWith('CVE-')) {
    return `https://nvd.nist.gov/vuln/detail/${alias}`;
  } else if (alias.startsWith('GHSA-')) {
    return `https://github.com/advisories/${alias}`;
  }
  return null;
}

function cvssUrl(cvss) {
  // Extract version from CVSS string (e.g., "CVSS:3.1/..." -> "3.1")
  let match = cvss.match(/^CVSS:(\d+\.\d+)\//);
  if (match) {
    return `https://www.first.org/cvss/calculator/${match[1]}#${cvss}`;
  }
  return null;
}

<template>
  <CrateHeader @crate={{@controller.crate}} />
  {{#if @controller.advisories.length}}
    <h2 class='heading'>Advisories</h2>
    <ul class='advisories' data-test-list>
      {{#each @controller.advisories as |advisory|}}
        <li class='row'>
          <h3>
            <a href='https://rustsec.org/advisories/{{advisory.id}}.html'>{{advisory.id}}</a>:
            {{advisory.summary}}
          </h3>
          {{#if advisory.versionRanges}}
            <div class='affected-versions' data-test-affected-versions>
              <strong>Affected versions:</strong>
              {{advisory.versionRanges}}
            </div>
          {{/if}}
          {{#if advisory.aliases.length}}
            <div class='aliases' data-test-aliases>
              <strong>Aliases:</strong>
              <ul>
                {{#each advisory.aliases as |alias|}}
                  <li><a href={{aliasUrl alias}}>{{alias}}</a></li>
                {{/each}}
              </ul>
            </div>
          {{/if}}
          {{#if advisory.cvss}}
            {{#let (calculateCvssScore advisory.cvss) as |score|}}
              <div class='cvss' data-test-cvss>
                <strong>CVSS:</strong>
                {{#if score}}
                  <span class={{severityClass score}}>{{score}} ({{severityRating score}})</span>
                  —
                {{/if}}
                <a href={{cvssUrl advisory.cvss}}>{{advisory.cvss}}</a>
              </div>
            {{/let}}
          {{/if}}
          {{htmlSafe (@controller.convertMarkdown advisory.details)}}
        </li>
      {{/each}}
    </ul>
  {{else}}
    <div class='no-results' data-no-advisories>
      No advisories found for this crate.
    </div>
  {{/if}}
</template>
