"""Prototype-only mechanical evidence validation; no semantic truth claims."""
import hashlib
import json


class InvalidReview(ValueError):
    pass


def require(condition, message):
    if not condition:
        raise InvalidReview(message)


def text(value):
    return isinstance(value, str) and bool(value.strip())


def keys(value, expected):
    require(isinstance(value, dict) and set(value) == set(expected),
            "unexpected or missing object fields")


def strings(value):
    require(isinstance(value, list) and all(text(v) for v in value),
            "expected a list of nonempty strings")


def parse(raw):
    require(isinstance(raw, str) and len(raw.encode('utf-8')) <= 128_000,
            "invalid or oversized response")

    def unique(pairs):
        result = {}
        for key, value in pairs:
            require(key not in result, "duplicate JSON key")
            result[key] = value
        return result

    try:
        return json.loads(raw, object_pairs_hook=unique)
    except (ValueError, RecursionError) as exc:
        raise InvalidReview("invalid JSON: " + str(exc)) from exc


def packet(case):
    keys(case, ('id', 'request', 'subject', 'documents'))
    require(all(text(case[k]) for k in ('id', 'request', 'subject')), "invalid case identity")
    require(isinstance(case['documents'], list), "documents must be an array")
    ids = set()
    for doc in case['documents']:
        keys(doc, ('id', 'path', 'revision', 'content'))
        require(all(text(v) for v in doc.values()), "invalid source metadata")
        require(doc['id'] not in ids, "duplicate source id")
        ids.add(doc['id'])
    # Includes the request, revision labels, source text and ordering, not just IDs.
    encoded = json.dumps(case, sort_keys=True, ensure_ascii=False, separators=(',', ':')).encode()
    return {'snapshot_id': hashlib.sha256(encoded).hexdigest(), **case}


def citation(ref, sources):
    keys(ref, ('source', 'quote'))
    require(text(ref['source']) and ref['source'] in sources, "unknown source")
    require(text(ref['quote']), "empty excerpt")
    require(ref['quote'] in sources[ref['source']]['content'], "excerpt not found in source")


def citations(refs, sources):
    require(isinstance(refs, list) and bool(refs), "evidence must be nonempty")
    for ref in refs:
        citation(ref, sources)


def common(review, evidence, field):
    keys(evidence, ('snapshot_id', 'id', 'request', 'subject', 'documents'))
    canonical = packet({k: v for k, v in evidence.items() if k != 'snapshot_id'})
    require(canonical['snapshot_id'] == evidence['snapshot_id'], "evidence packet changed")
    keys(review, ('snapshot_id', field, 'limitations'))
    require(review['snapshot_id'] == evidence['snapshot_id'], "stale or wrong snapshot")
    require(isinstance(review[field], list), "entries must be an array")
    strings(review['limitations'])
    return {doc['id']: doc for doc in evidence['documents']}


def validate_candidates(review, evidence):
    sources = common(review, evidence, 'candidates')
    seen = set()
    for finding in review['candidates']:
        keys(finding, ('id', 'claim', 'requirement', 'evidence', 'basis', 'assumptions', 'question'))
        require(text(finding['id']) and finding['id'] not in seen, "invalid/duplicate finding id")
        seen.add(finding['id'])
        require(text(finding['claim']) and text(finding['question']), "empty claim/question")
        require(finding['basis'] in ('source', 'execution_record'), "invalid evidence basis")
        strings(finding['assumptions'])
        citation(finding['requirement'], sources)
        citations(finding['evidence'], sources)


def validate_assessments(review, evidence, candidates):
    validate_candidates(candidates, evidence)
    sources = common(review, evidence, 'assessments')
    expected = {item['id'] for item in candidates['candidates']}
    seen = set()
    for item in review['assessments']:
        keys(item, ('id', 'disposition', 'rationale', 'evidence', 'final_claim'))
        require(text(item['id']) and item['id'] in expected and item['id'] not in seen,
                "unknown or duplicate assessment id")
        seen.add(item['id'])
        require(item['disposition'] in ('supported', 'not_supported', 'uncertain'),
                "invalid advisory disposition")
        require(text(item['rationale']), "empty rationale")
        citations(item['evidence'], sources)
        if item['disposition'] == 'supported':
            require(text(item['final_claim']), "supported claim is missing")
        else:
            require(item['final_claim'] is None, "unconfirmed claim cannot be rendered as finding")
    require(seen == expected, "missing candidate assessments")


def render(review, evidence, candidates):
    validate_assessments(review, evidence, candidates)
    # No prose rewrite: it could introduce claims that were never verified.
    # Mechanical success does NOT establish the truth of these model judgments.
    sources = {doc['id']: doc for doc in evidence['documents']}
    lines = ['Advisory consistency review', 'Subject: ' + evidence['subject'],
             'Snapshot: ' + evidence['snapshot_id'],
             'Mechanical checks passed; semantic judgments remain model assessments.']
    supported = [v for v in review['assessments'] if v['disposition'] == 'supported']
    lines.append('Supported findings:')
    if not supported:
        lines.append('No supported findings returned; this does not establish complete inspection or qualification.')
    for item in supported:
        lines.append(item['id'] + ': ' + item['final_claim'])
        lines.append('Evidence: ' + '; '.join(sources[r['source']]['path'] + ' @ ' +
                                            sources[r['source']]['revision'] for r in item['evidence']))
    unresolved = [v for v in review['assessments'] if v['disposition'] == 'uncertain']
    lines.append('Unresolved questions:')
    for item in unresolved:
        lines.append(item['id'] + ': ' + item['rationale'])
    if not unresolved:
        lines.append('None returned.')
    lines.append('Candidates not supported: ' + ', '.join(v['id'] for v in review['assessments']
                                                       if v['disposition'] == 'not_supported'))
    lines.append('Inspection limits:')
    lines.extend(dict.fromkeys(candidates['limitations'] + review['limitations']))
    lines.append('No core gate, adoption, merge or release approval is granted.')
    return '\n'.join(lines)
