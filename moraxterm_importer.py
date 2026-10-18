#!/usr/bin/env python3
"""Convert read-only Moraxterm INI sections into safe Session objects."""
from __future__ import annotations

import re
from pathlib import Path
from typing import Iterable

from moraxterm_config import Section, parse_ini
from session_model import Session, SessionValidationError, session_from_fields

_SECRET_KEY_PARTS = ('password', 'passphrase', 'secret', 'token', 'privatekey', 'private_key')
_TYPE_ALIASES = {
    'ssh': 'SSH', 'sftp': 'Sftp', 'scp': 'Sftp', 'telnet': 'Telnet',
    'rsh': 'Rsh', 'xdmcp': 'Xdmcp', 'rdp': 'Rdp', 'vnc': 'Vnc',
    'ftp': 'Ftp', 'serial': 'Serial', 'mosh': 'Mosh', 'shell': 'Shell',
    'wsl': 'WSL', 'terminal': 'Terminal',
}


def _key(value: str) -> str:
    return re.sub(r'[^a-z0-9]', '', value.lower())


def _bool(value: str) -> bool:
    return value.strip().lower() in {'1', 'true', 'yes', 'on', 'enabled'}


def _find(values: dict[str, str], *names: str) -> str | None:
    wanted = {_key(name) for name in names}
    for key, value in values.items():
        if _key(key) in wanted:
            return value
    return None


def _infer_type(section: Section, values: dict[str, str]) -> str:
    explicit = _find(values, 'SessionType', 'Type', 'Protocol')
    candidates = [explicit or '', section.name]
    for candidate in candidates:
        token = _key(candidate).replace('session', '')
        for alias, session_type in _TYPE_ALIASES.items():
            if token == alias or token.endswith(alias):
                return session_type
    raise ValueError(f"unsupported Moraxterm session type in section {section.name!r}")


def _safe_values(section: Section) -> dict[str, str]:
    return {
        item.name: item.value
        for item in section.values
        if not item.redacted and not any(part in _key(item.name) for part in _SECRET_KEY_PARTS)
    }


def _bookmark_session(name: str, encoded: str) -> Session:
    match = re.match(r'^#(\d+)#(\d+)(?:%(.*))?$', encoded)
    if not match:
        raise ValueError(f"invalid bookmark record for {name!r}")
    _type_code, mode, remainder = match.groups()
    fields = [remainder] if remainder is None else remainder.split('%')
    if mode == '4':
        session_type = 'Rdp'
    elif mode == '14':
        session_type = 'WSL'
    elif mode == '0':
        session_type = 'SSH'
    elif mode == '5':
        session_type = 'Vnc'
    elif mode == '7':
        session_type = 'SSH'
    elif mode == '14':
        session_type = 'WSL'
    else:
        raise ValueError(f"unsupported bookmark mode {mode!r} for {name!r}")
    if session_type == 'WSL':
        options = {'distribution_id': fields[0]} if fields and fields[0] else {}
        return session_from_fields(name, 'WSL', {'options': options})
    if len(fields) < 3 or not fields[0] or not fields[1]:
        raise ValueError(f"bookmark {name!r} lacks host or port")
    gateway = {}
    if (
        len(fields) > 9
        and fields[8]
        and fields[8] != '-1'
        and fields[9]
        and re.fullmatch(r'\d+', fields[9])
    ):
        gateway = {'host': fields[8], 'port': int(fields[9])}
        if len(fields) > 10 and fields[10] and fields[10] != '-1':
            gateway['username'] = fields[10]
    return session_from_fields(name, session_type, {
        'host': fields[0],
        'port': fields[1],
        'username': fields[2],
        'gateway': gateway,
    })


def _convert(section: Section) -> Session:
    values = _safe_values(section)
    session_type = _infer_type(section, values)
    options: dict[str, bool] = {}
    option_map = {
        'Compression': 'compression',
        'X11Forwarding': 'x11_forwarding',
        'AgentForwarding': 'agent_forwarding',
    }
    for source, target in option_map.items():
        raw = _find(values, source)
        if raw is not None:
            options[target] = _bool(raw)
    fields = {
        'host': _find(values, 'Host', 'RemoteHost', 'Server'),
        'username': _find(values, 'User', 'Username', 'Login'),
        'port': _find(values, 'Port', 'RemotePort'),
        'command': _find(values, 'Command', 'ExecuteCommand', 'StartupCommand'),
        'options': options,
    }
    fields = {key: value for key, value in fields.items() if value not in (None, '')}
    name = section.name.removeprefix('Session').strip() or section.name
    try:
        return session_from_fields(name, session_type, fields)
    except (ValueError, SessionValidationError) as exc:
        raise ValueError(f"invalid session section {section.name!r}: {exc}") from exc


def import_sections(sections: Iterable[Section]) -> list[Session]:
    """Import explicit session sections and encoded bookmark records."""
    sessions: list[Session] = []
    for section in sections:
        if section.name == 'Bookmarks' or section.name.startswith('Bookmarks_'):
            for item in section.values:
                if item.name in {'SubRep', 'ImgNum'} or not item.value.startswith('#'):
                    continue
                sessions.append(_bookmark_session(item.name, item.value))
        elif section.kind == 'session' and section.name.lower().startswith('session'):
            sessions.append(_convert(section))
    return sessions


def import_ini(path: str | Path) -> list[Session]:
    return import_sections(parse_ini(path))
