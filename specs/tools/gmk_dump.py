#!/usr/bin/env python3
"""Reference decoder for the formats in specs/file-formats.md.

Written from the spec (clean-room). Parses a GameMaker project file end-to-end and
prints JSON. Every zlib block and the file itself must be consumed exactly; any
leftover or missing byte raises FormatError.

Usage:
    python -I gmk_dump.py FILE.gmk|FILE.gm81  [--blobs]   project (v701/v800/v810*)
    python -I gmk_dump.py FILE.lib                        action library
    python -I gmk_dump.py FILE.ged                        extension package description
    python -I gmk_dump.py FILE.dat                        extension payload (lists chunks)
(* v810 is parsed with the v800 layout; see spec section 7.)

Binary payloads (images, sounds, icons) are summarised as {"len", "sha1"} unless
--blobs is given, in which case they are base64-encoded.
"""
import base64
import hashlib
import json
import struct
import sys
import zlib

MAGIC = 1234321


class FormatError(Exception):
    pass


class Stream:
    def __init__(self, data, base=0, what='file'):
        self.d = data
        self.p = 0
        self.what = what

    def _take(self, n):
        if n < 0 or self.p + n > len(self.d):
            raise FormatError('%s: read of %d bytes at %d overruns %d' % (self.what, n, self.p, len(self.d)))
        b = self.d[self.p:self.p + n]
        self.p += n
        return b

    def i32(self):
        return struct.unpack('<i', self._take(4))[0]

    def u32(self):
        return struct.unpack('<I', self._take(4))[0]

    def bool(self):
        return self.i32() != 0

    def f64(self):
        return struct.unpack('<d', self._take(8))[0]

    time = f64

    def raw(self, n):
        return self._take(n)

    def str(self):
        return self._take(self.u32()).decode('cp1252', errors='replace')

    def blob(self):
        return self._take(self.u32())

    def zblock(self, what):
        data = self.blob()
        try:
            return Stream(zlib.decompress(data), what=what)
        except zlib.error as e:
            raise FormatError('%s: bad zlib block: %s' % (what, e))

    def expect(self, value, what):
        v = self.i32()
        if v != value:
            raise FormatError('%s: expected %d, got %d at %d' % (what, value, v, self.p - 4))
        return v

    def end(self):
        if self.p != len(self.d):
            raise FormatError('%s: %d unconsumed bytes at %d' % (self.what, len(self.d) - self.p, self.p))


BLOBS = False


def blob_repr(b):
    if BLOBS:
        return {'len': len(b), 'base64': base64.b64encode(b).decode('ascii')}
    return {'len': len(b), 'sha1': hashlib.sha1(b).hexdigest()}


# ---------------------------------------------------------------- v701 envelope

def make_table(seed):
    a = 6 + seed % 250
    b = seed // 250
    t = list(range(256))
    for i in range(1, 10001):
        j = (i * a + b) % 254 + 1
        t[j], t[j + 1] = t[j + 1], t[j]
    inv = [0] * 256
    for x, v in enumerate(t):
        inv[v] = x
    return t, inv


def decode_701(data):
    s = Stream(data)
    s.expect(MAGIC, 'magic')
    s.expect(701, 'version')
    n1 = s.u32()
    n2 = s.u32()
    s.raw(4 * n1)
    seed = s.u32()
    s.raw(4 * n2)
    start = s.p
    _, inv = make_table(seed)
    # Spec 8.2: plain = (U[c] - absolute_offset) mod 256.  First byte: open question;
    # we decode it like the others.
    out = bytes((inv[data[p]] - p) & 0xFF for p in range(start, len(data)))
    return out, {'n1': n1, 'n2': n2, 'seed': seed, 'encoded_offset': start}


# ---------------------------------------------------------------- shared records

def img800(s):
    s.expect(800, 'image version')
    w = s.i32()
    h = s.i32()
    d = {'width': w, 'height': h}
    if w * h > 0:
        px = s.blob()
        if len(px) != w * h * 4:
            raise FormatError('image pixel blob size %d != %d' % (len(px), w * h * 4))
        d['bgra'] = blob_repr(px)
    return d


def img701(s):
    marker = s.i32()
    if marker == -1:
        return None
    if marker != 10:
        raise FormatError('img701 marker %d' % marker)
    z = s.blob()
    bmp = zlib.decompress(z)
    return {'bmp': blob_repr(bmp)}


def action_list(s):
    s.expect(400, 'action list version')
    acts = []
    for _ in range(s.i32()):
        a = {}
        a['version'] = s.expect(440, 'action version')
        a['library_id'] = s.i32()
        a['action_id'] = s.i32()
        a['kind'] = s.i32()
        a['may_be_relative'] = s.bool()
        a['is_question'] = s.bool()
        a['has_applies_to'] = s.bool()
        a['exe_type'] = s.i32()
        a['function_name'] = s.str()
        a['code'] = s.str()
        a['argument_count'] = s.i32()
        a['arg_kinds'] = [s.i32() for _ in range(s.i32())]
        a['applies_to'] = s.i32()
        a['relative'] = s.bool()
        a['args'] = [s.str() for _ in range(s.i32())]
        a['not'] = s.bool()
        acts.append(a)
    return acts


def obj_body(s):
    o = {}
    for k in ('sprite',):
        o[k] = s.i32()
    o['solid'] = s.bool()
    o['visible'] = s.bool()
    o['depth'] = s.i32()
    o['persistent'] = s.bool()
    o['parent'] = s.i32()
    o['mask_sprite'] = s.i32()
    o['max_event_type'] = s.i32()
    events = []
    for etype in range(o['max_event_type'] + 1):
        while True:
            num = s.i32()
            if num == -1:
                break
            events.append({'type': etype, 'number': num, 'actions': action_list(s)})
    o['events'] = events
    return o


def room_body(s):
    r = {'caption': s.str()}
    for k in ('width', 'height', 'snap_x', 'snap_y'):
        r[k] = s.i32()
    r['isometric_grid'] = s.bool()
    r['speed'] = s.i32()
    r['persistent'] = s.bool()
    r['background_color'] = s.i32()
    r['draw_background_color'] = s.bool()
    r['creation_code'] = s.str()
    bnames = ('visible', 'is_foreground', 'background', 'x', 'y', 'tile_h', 'tile_v', 'hspeed', 'vspeed', 'stretch')
    r['backgrounds'] = [dict(zip(bnames, [s.i32() for _ in bnames])) for _ in range(s.i32())]
    r['views_enabled'] = s.bool()
    vnames = ('visible', 'view_x', 'view_y', 'view_w', 'view_h', 'port_x', 'port_y', 'port_w', 'port_h',
              'h_border', 'v_border', 'h_speed', 'v_speed', 'follow_object')
    r['views'] = [dict(zip(vnames, [s.i32() for _ in vnames])) for _ in range(s.i32())]
    insts = []
    for _ in range(s.i32()):
        insts.append({'x': s.i32(), 'y': s.i32(), 'object': s.i32(), 'id': s.i32(),
                      'creation_code': s.str(), 'locked': s.bool()})
    r['instances'] = insts
    tnames = ('x', 'y', 'background', 'tile_x', 'tile_y', 'width', 'height', 'depth', 'id')
    tiles = []
    for _ in range(s.i32()):
        t = dict(zip(tnames, [s.i32() for _ in tnames]))
        t['locked'] = s.bool()
        tiles.append(t)
    r['tiles'] = tiles
    enames = ('remember_room_editor_info', 'editor_width', 'editor_height', 'show_grid', 'show_objects',
              'show_tiles', 'show_backgrounds', 'show_foregrounds', 'show_views', 'delete_underlying_objects',
              'delete_underlying_tiles', 'current_tab', 'h_scroll', 'v_scroll')
    r['editor_state'] = dict(zip(enames, [s.i32() for _ in enames]))
    return r


def path_body(s):
    p = {'connection_kind': s.i32(), 'closed': s.bool(), 'precision': s.i32(), 'backdrop_room': s.i32(),
         'snap_x': s.i32(), 'snap_y': s.i32()}
    p['points'] = [{'x': s.f64(), 'y': s.f64(), 'speed': s.f64()} for _ in range(s.i32())]
    return p


def timeline_body(s):
    return {'moments': [{'step': s.i32(), 'actions': action_list(s)} for _ in range(s.i32())]}


def font_body(s):
    return {'font_name': s.str(), 'size': s.i32(), 'bold': s.bool(), 'italic': s.bool(),
            'range_start': s.i32(), 'range_end': s.i32()}


# ---------------------------------------------------------------- v800 bodies

def sound800(s):
    d = {'kind': s.i32(), 'file_type': s.str(), 'file_name': s.str()}
    if s.bool():
        d['data'] = blob_repr(s.blob())
    d['effects'] = s.i32()
    d['volume'] = s.f64()
    d['pan'] = s.f64()
    d['preload'] = s.bool()
    return d


def sprite800(s):
    d = {'origin_x': s.i32(), 'origin_y': s.i32()}
    n = s.i32()
    d['frames'] = [img800(s) for _ in range(n)]
    # Trailer is present even when n == 0 (verified on user data, see spec addendum).
    d['collision_shape'] = s.i32()
    d['alpha_tolerance'] = s.i32()
    d['separate_collision_masks'] = s.bool()
    d['bbox_mode'] = s.i32()
    d['bbox_left'] = s.i32()
    d['bbox_right'] = s.i32()
    d['bbox_bottom'] = s.i32()
    d['bbox_top'] = s.i32()
    return d


def background800(s):
    d = {'use_as_tileset': s.bool()}
    for k in ('tile_width', 'tile_height', 'tile_h_offset', 'tile_v_offset', 'tile_h_sep', 'tile_v_sep'):
        d[k] = s.i32()
    d['image'] = img800(s)
    return d


def script_body(s):
    return {'code': s.str()}


def settings800(s, v800=True):
    st = {}
    names = ['start_fullscreen', 'interpolate_pixels', 'dont_draw_border', 'display_cursor', 'scaling',
             'allow_window_resize', 'always_on_top', 'color_outside_room', 'set_resolution', 'color_depth',
             'resolution', 'frequency', 'dont_show_buttons', 'vsync']
    if v800:
        names.append('disable_screensaver')
    names += ['let_f4_fullscreen', 'let_f1_game_info', 'let_esc_end_game', 'let_f5_save_f6_load',
              'let_f9_screenshot', 'treat_close_as_esc', 'process_priority', 'freeze_on_lose_focus', 'loading_bar']
    for n in names:
        st[n] = s.i32()
    if st['loading_bar'] == 2:  # spec: [I]
        if s.bool():
            st['loading_bar_back'] = blob_repr(s.blob())
        if s.bool():
            st['loading_bar_front'] = blob_repr(s.blob())
    st['show_custom_load_image'] = s.bool()
    if st['show_custom_load_image']:  # spec: [I]
        if s.bool():
            st['custom_load_image'] = blob_repr(s.blob())
    st['load_image_transparent'] = s.bool()
    st['load_image_alpha'] = s.i32()
    st['scale_progress_bar'] = s.bool()
    st['icon'] = blob_repr(s.blob())
    for n in ('display_errors', 'write_errors_to_log', 'abort_on_all_errors'):
        st[n] = s.bool()
    # i32 bit set, not a bool: bit 0 = treat uninitialized as 0; GM 8.1 also stores
    # bit 1 (value 2, meaning unverified) — see spec addendum.
    flags = s.i32()
    st['treat_uninitialized_as_zero'] = bool(flags & 1)
    st['uninitialized_extra_bits'] = flags & ~1
    st['author'] = s.str()
    st['version_text'] = s.str()
    st['info_last_changed'] = s.time()
    st['information'] = s.str()
    if not v800:
        st['constants'] = [{'name': s.str(), 'value': s.str()} for _ in range(s.i32())]
    st['version'] = [s.i32() for _ in range(4)]
    for n in ('company', 'product', 'copyright', 'description'):
        st[n] = s.str()
    if v800:
        st['settings_last_changed'] = s.time()
    return st


def tree(s):
    roots = []

    def node():
        n = {'status': s.i32(), 'group_kind': s.i32(), 'index': s.i32(), 'name': s.str()}
        n['children'] = [node() for _ in range(s.i32())]
        return n

    for _ in range(12):
        roots.append(node())
    return roots


def trailing(s, v800, G):
    G['last_instance_id'] = s.i32()
    G['last_tile_id'] = s.i32()
    # included files
    inc = {'version': s.i32(), 'files': []}
    for i in range(s.i32()):
        if v800:
            b = s.zblock('include %d' % i)
        else:
            b = s
        f = {}
        if v800:
            f['last_changed'] = b.time()
        f['version'] = b.i32()
        f['file_name'] = b.str()
        f['original_path'] = b.str()
        f['original_file_chosen'] = b.bool()
        f['original_size'] = b.i32()
        f['store_in_editable'] = b.bool()
        if f['store_in_editable']:
            f['data'] = blob_repr(b.blob())
        f['export_kind'] = b.i32()
        f['export_folder'] = b.str()
        f['overwrite_existing'] = b.bool()
        f['free_memory_after_export'] = b.bool()
        f['remove_at_game_end'] = b.bool()
        if v800:
            b.end()
        inc['files'].append(f)
    G['included_files'] = inc
    G['extension_packages'] = {'version': s.i32(), 'names': [s.str() for _ in range(s.i32())]}
    gv = s.i32()
    g = s.zblock('game information') if v800 else s
    gi = {'version': gv, 'background_color': g.i32() & 0xFFFFFFFF, 'show_in_separate_window': g.bool(),
          'window_caption': g.str(), 'left': g.i32(), 'top': g.i32(), 'width': g.i32(), 'height': g.i32(),
          'show_window_border': g.bool(), 'allow_resize': g.bool(), 'stay_on_top': g.bool(),
          'pause_game': g.bool()}
    if v800:
        gi['last_changed'] = g.time()
    gi['rtf'] = g.str()
    if v800:
        g.end()
    G['game_information'] = gi
    G['library_creation_code'] = {'version': s.i32(), 'code': [s.str() for _ in range(s.i32())]}
    G['room_order'] = {'version': s.i32(), 'rooms': [s.i32() for _ in range(s.i32())]}
    G['resource_tree'] = tree(s)


def parse_gmk(data):
    head = Stream(data)
    head.expect(MAGIC, 'magic')
    version = head.i32()
    G = {'format': 'gmk', 'version': version}
    if version == 701:
        plain, env = decode_701(data)
        G['envelope_701'] = env
        s = Stream(plain, what='decoded stream')
        v800 = False
    elif version in (800, 810):
        s = head
        v800 = True
    else:
        raise FormatError('unsupported project version %d' % version)

    G['game_id'] = s.i32()
    G['guid'] = s.raw(16).hex()
    G['settings_version'] = s.i32()
    if v800:
        st = s.zblock('settings')
        G['settings'] = settings800(st, True)
        st.end()
        G['triggers_version'] = s.i32()
        trig = []
        for i in range(s.i32()):
            b = s.zblock('trigger %d' % i)
            if not b.bool():
                trig.append(None)
            else:
                trig.append({'version': b.i32(), 'name': b.str(), 'condition': b.str(), 'moment': b.i32(),
                             'constant_name': b.str()})
            b.end()
        G['triggers'] = trig
        G['triggers_last_changed'] = s.time()
        G['constants_version'] = s.i32()
        G['constants'] = [{'name': s.str(), 'value': s.str()} for _ in range(s.i32())]
        G['constants_last_changed'] = s.time()
    else:
        G['settings'] = settings800(s, False)

    if v800:
        bodies = [('sounds', sound800), ('sprites', sprite800), ('backgrounds', background800),
                  ('paths', path_body), ('scripts', script_body), ('fonts', font_body),
                  ('timelines', timeline_body), ('objects', obj_body), ('rooms', room_body)]
    else:
        bodies = [('sounds', sound701), ('sprites', sprite701), ('backgrounds', background701),
                  ('paths', path_body), ('scripts', script_body), ('fonts', font_body),
                  ('timelines', timeline_body), ('objects', obj_body), ('rooms', room_body)]
    for label, fn in bodies:
        lst = {'version': s.i32(), 'items': []}
        for i in range(s.i32()):
            b = s.zblock('%s[%d]' % (label, i)) if v800 else s
            if not b.bool():
                lst['items'].append(None)
            else:
                item = {'name': b.str()}
                if v800:
                    item['last_changed'] = b.time()
                item['resource_version'] = b.i32()
                item.update(fn(b))
                lst['items'].append(item)
            if v800:
                b.end()
        G[label] = lst
    trailing(s, v800, G)
    s.end()
    return G


# ---------------------------------------------------------------- v701 bodies

def sound701(s):
    d = {'kind': s.i32(), 'file_type': s.str(), 'file_name': s.str()}
    if s.bool():
        d['data'] = blob_repr(zlib.decompress(s.blob()))
    d['effects'] = s.i32()
    d['volume'] = s.f64()
    d['pan'] = s.f64()
    d['preload'] = s.bool()
    return d


def sprite701(s):
    d = {}
    for k in ('width', 'height', 'bbox_left', 'bbox_right', 'bbox_bottom', 'bbox_top'):
        d[k] = s.i32()
    for k in ('transparent', 'smooth_edges', 'preload'):
        d[k] = s.bool()
    d['bbox_mode'] = s.i32()
    d['precise_collision'] = s.bool()
    d['origin_x'] = s.i32()
    d['origin_y'] = s.i32()
    d['frames'] = [img701(s) for _ in range(s.i32())]
    return d


def background701(s):
    d = {'width': s.i32(), 'height': s.i32()}
    for k in ('transparent', 'smooth_edges', 'preload', 'use_as_tileset'):
        d[k] = s.bool()
    for k in ('tile_width', 'tile_height', 'tile_h_offset', 'tile_v_offset', 'tile_h_sep', 'tile_v_sep'):
        d[k] = s.i32()
    d['image'] = img701(s) if s.bool() else None
    return d


# ---------------------------------------------------------------- .lib / .ged / .dat

def parse_lib(data):
    s = Stream(data)
    L = {'format': 'lib', 'format_version': s.expect(520, 'lib version'), 'tab_caption': s.str(),
         'library_id': s.i32(), 'author': s.str(), 'library_version': s.i32(), 'last_changed': s.time(),
         'info': s.str(), 'init_code': s.str(), 'advanced_only': s.bool(), 'editor_counter': s.i32()}
    acts = []
    for _ in range(s.i32()):
        a = {'version': s.expect(520, 'lib action version'), 'name': s.str(), 'action_id': s.i32(),
             'image': blob_repr(s.blob()), 'hidden': s.bool(), 'advanced_only': s.bool(), 'pro_only': s.bool(),
             'palette_title': s.str(), 'list_text': s.str(), 'hint_text': s.str(), 'kind': s.i32(),
             'interface': s.i32(), 'is_question': s.bool(), 'show_apply_to': s.bool(),
             'show_relative': s.bool(), 'argument_count': s.i32()}
        a['arg_slots'] = [{'caption': s.str(), 'kind': s.i32(), 'default': s.str(), 'menu_items': s.str()}
                          for _ in range(s.i32())]
        a['exe_type'] = s.i32()
        a['function_name'] = s.str()
        a['code'] = s.str()
        acts.append(a)
    L['actions'] = acts
    s.end()
    return L


def parse_ged(data):
    s = Stream(data)
    g = {'format': 'ged', 'version': s.expect(700, 'ged version'), 'editable': s.bool(), 'name': s.str(),
         'folder': s.str(), 'version_text': s.str(), 'author': s.str(), 'date': s.str(), 'license': s.str(),
         'description': s.str(), 'help_file': s.str(), 'hidden': s.bool()}
    g['uses'] = [s.str() for _ in range(s.i32())]
    files = []
    for _ in range(s.i32()):
        f = {'version': s.expect(700, 'ged file version'), 'file_name': s.str(), 'original_path': s.str(),
             'file_kind': s.i32(), 'init_function': s.str(), 'final_function': s.str()}
        funcs = []
        for _ in range(s.i32()):
            fu = {'version': s.expect(700, 'ged function version'), 'name': s.str(), 'external_name': s.str(),
                  'call_convention': s.i32(), 'help_line': s.str(), 'hidden': s.bool(),
                  'argument_count': s.i32()}
            fu['arg_types'] = [s.i32() for _ in range(17)]
            fu['return_type'] = s.i32()
            funcs.append(fu)
        f['functions'] = funcs
        f['constants'] = [{'version': s.expect(700, 'ged constant version'), 'name': s.str(), 'value': s.str(),
                           'hidden': s.bool()} for _ in range(s.i32())]
        files.append(f)
    g['files'] = files
    s.end()
    return g


def parse_dat(data):
    seed = struct.unpack_from('<I', data, 0)[0]
    _, inv = make_table(seed)
    enc = data[4:]
    plain = bytes([enc[0]]) + bytes(inv[c] for c in enc[1:])
    s = Stream(plain, what='dat payload')
    chunks = []
    while s.p < len(plain):
        content = zlib.decompress(s.blob())
        chunks.append(blob_repr(content))
    s.end()
    return {'format': 'dat', 'seed': seed, 'chunks': chunks}


def main(argv):
    global BLOBS
    args = [a for a in argv[1:] if not a.startswith('--')]
    BLOBS = '--blobs' in argv
    if len(args) != 1:
        print(__doc__)
        return 2
    path = args[0]
    data = open(path, 'rb').read()
    low = path.lower()
    if low.endswith('.lib'):
        out = parse_lib(data)
    elif low.endswith('.ged'):
        out = parse_ged(data)
    elif low.endswith('.dat'):
        out = parse_dat(data)
    else:
        out = parse_gmk(data)
    json.dump(out, sys.stdout, indent=1, ensure_ascii=False)
    sys.stdout.write('\n')
    return 0


if __name__ == '__main__':
    sys.exit(main(sys.argv))
