"""Build private Minecraft resources from the source export. No retail assets are tracked."""
from pathlib import Path
from import_facades import export_facades
from nombres_es import nombre_es
import gzip,hashlib,json,math,shutil,struct

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'.local/content'
DEST=ROOT/'src/generated/resources'
SITES=('swamp_hut','snow_fort','sandover','crater_cave')

def packed_blueprint(data):
    out=bytearray(b'JAKB\x00\x01')
    out.extend(struct.pack('>I',len(data['palette'])))
    for state in data['palette']:
        encoded=state.encode('utf-8');out.extend(struct.pack('>H',len(encoded)));out.extend(encoded)
    out.extend(struct.pack('>I',len(data['blocks'])))
    for x,y,z,palette in data['blocks']:
        if not all(-32768<=v<32768 for v in (x,y,z)) or not 0<=palette<65536:raise ValueError('Blueprint field overflow')
        out.extend(struct.pack('>hhhH',x,y,z,palette))
    props=json.dumps(data.get('props',[]),separators=(',',':')).encode('utf-8')
    out.extend(struct.pack('>I',len(props)));out.extend(props)
    return gzip.compress(out,mtime=0)

NATIVE_MATERIALS={
    'oak_door','oak_trapdoor','spruce_trapdoor','oak_stairs','stone_stairs',
    'oak_slab','smooth_quartz_slab','spruce_slab','crimson_slab','cobblestone_wall',
    'glass_pane','iron_bars','lightning_rod','leaf_litter','ice','sea_lantern','stone',
}

def authored_boxes(path):
    model=DEST/'assets/jak/models/block'/f'{path}.json'
    if not model.is_file():return None
    elements=json.loads(model.read_text(encoding='utf-8')).get('elements',[])
    if not elements or any('rotation' in e or any(v<0 or v>16 for v in e['from']+e['to']) for e in elements):
        return None
    return [e['from']+e['to'] for e in elements]

GATE='krimson_porton_fence_gate'
GATE_HEIGHT=4

# Decorative parts with a single placement cell in the city plan. Keep their
# original faces and UVs while fitting the occupied dimensions to that cell.
FITTED_ARCHITECTURE={
    'haven_canes','krimson_blindaje','krimson_franja','krimson_dintel',
    'krimson_insignia','krimson_puesto','barrio_cartel','barrio_cartel_b',
    'haven_estandarte','barrio_codo','barrio_desague','haven_chimenea_remate',
    'haven_cupula_madera','haven_poste_fachada','haven_tejadillo','haven_toldo_tela',
    'sparg_costilla','sparg_columna',
}

def fit_architecture():
    source=SOURCE/'source-pack/assets/jak/models/block'
    target=DEST/'assets/jak/models/block'
    for name in sorted(FITTED_ARCHITECTURE):
        data=json.loads((source/(name+'.json')).read_text(encoding='utf-8'))
        elements=data.get('elements',[])
        if not elements or any('rotation' in e for e in elements):
            continue
        for axis in range(3):
            low=min(e['from'][axis] for e in elements)
            high=max(e['to'][axis] for e in elements)
            if low>=0 and high<=16:
                continue
            scale=min(1,16/(high-low))
            start=min(max(low,0),16-(high-low)*scale)
            for element in elements:
                for corner in ('from','to'):
                    element[corner][axis]=round(min(16,max(0,(element[corner][axis]-low)*scale+start)),5)
        (target/(name+'.json')).write_text(json.dumps(data,separators=(',',':')),encoding='utf-8')

def gate_model(source,section,opened):
    out={'textures':source['textures'],'elements':[]}
    for element in source['elements']:
        start,end=element['from'],element['to']
        low,high=max(start[1],section*16),min(end[1],(section+1)*16)
        if low>=high:continue
        z_offset=3 if opened else 0
        faces={}
        for side,face in element['faces'].items():
            if side=='up' and high<end[1] or side=='down' and low>start[1]:continue
            face=face.copy()
            if side not in ('up','down'):
                u0,v0,u1,v1=face['uv']
                t0=(end[1]-high)/(end[1]-start[1])
                t1=(end[1]-low)/(end[1]-start[1])
                face['uv']=[u0,v0+(v1-v0)*t0,u1,v0+(v1-v0)*t1]
            faces[side]=face
        out['elements'].append({'from':[start[0],low-section*16,start[2]+z_offset],
                                'to':[end[0],high-section*16,end[2]+z_offset],'faces':faces})
    return out

def gate_resources():
    source=SOURCE/'source-pack/assets/jak/models/block'
    target=DEST/'assets/jak'
    by_open=[]
    for opened in (False,True):
        name=GATE+('_open' if opened else '')
        original=json.loads((source/(name+'.json')).read_text(encoding='utf-8'))
        sections=[gate_model(original,section,opened) for section in range(GATE_HEIGHT)]
        for section,model in enumerate(sections):
            (target/'models/block'/f'{name}_{section}.json').write_text(json.dumps(model),encoding='utf-8')
        by_open.append([[e['from']+e['to'] for e in model['elements']] for model in sections])
    multipart=[]
    post_sources=[]
    style=json.loads((source/(GATE+'.json')).read_text(encoding='utf-8'))
    for side,source_rail in zip(('left','right'),style['elements'][-2:]):
        for top in (False,True):
            name=f'{GATE}_jamb_{side}'+('_top' if top else '')
            low=list(source_rail['from']);high=list(source_rail['to'])
            low[1],high[1]=0,16
            if top:low[2]=0
            rail={'textures':style['textures'],
                  'elements':[{'from':low,'to':high,'faces':source_rail['faces']}]}
            (target/'models/block'/f'{name}.json').write_text(json.dumps(rail),encoding='utf-8')
            post_sources.append((side,top,name))
    for direction,angle in {'south':0,'west':90,'north':180,'east':270}.items():
        for opened in (False,True):
            for section in range(GATE_HEIGHT):
                name=GATE+('_open' if opened else '')+f'_{section}'
                multipart.append({'when':{'facing':direction,'open':str(opened).lower(),'section':str(section)},
                                  'apply':{'model':'jak:block/'+name,'y':angle,'uvlock':True}})
        for side,top,name in post_sources:
            multipart.append({'when':{'facing':direction,'open':'true','section':'3' if top else '0|1|2',
                                      side+'_jamb':'true'},
                              'apply':{'model':'jak:block/'+name,'y':angle,'uvlock':True}})
    (target/'blockstates'/f'{GATE}.json').write_text(json.dumps({'multipart':multipart}),encoding='utf-8')
    return by_open

# Adrii 06/11: the palace's small rocks are a cutout cube of a few grey shards, no use as a block.
REMOVED={'sp_waspala-small-rocks'}

def has_holes(path):
    model=DEST/'assets/jak/models/block'/(path+'.json')
    if not model.is_file():return False
    from PIL import Image
    for texture in set(json.loads(model.read_text(encoding='utf-8')).get('textures',{}).values()):
        image=DEST/'assets/jak/textures/block'/(texture.split('/')[-1]+'.png')
        if image.is_file() and Image.open(image).convert('RGBA').getextrema()[3][0]<128:return True
    return False

def configure_materials(definitions):
    for row in definitions:
        kind,vanilla=row['look']['kind'],row['look'].get('vanilla','')
        if vanilla in ('carved_pumpkin','stone','sea_lantern'):
            boxes=authored_boxes(row['path'])
            if boxes is not None:
                kind='authored' if vanilla=='carved_pumpkin' else 'static_authored'
                row['look']={'kind':kind,'boxes':boxes,'vanilla':vanilla}
        if row['path']==GATE:
            kind='gate'
            row['look']={'kind':kind,'parts':gate_resources()}
        row['ready']=not row['path'].startswith('objeto_') and (kind!='native' or vanilla in NATIVE_MATERIALS)
        if row['path'] in REMOVED:row['ready']=False
        # Adrii 06/11: a full block or slab whose texture has holes (the Haven foliage) still hid its neighbours'
        # faces, so the world showed through it. Such blocks are marked to leave them drawn.
        if kind in ('cube','slab','pillar','layers') and has_holes(row['path']):row['look']['see_through']=True

def material_resources(definitions):
    """Minecraft owns the loot transactions and partial block-state predicates."""
    loot=DEST/'data/jak/loot_table/blocks'
    loot.mkdir(parents=True,exist_ok=True)
    translations={}
    tags={name:[] for name in ('pickaxe','axe','shovel','hoe')}
    def loot_state(path,prop,value):
        return {'type':'minecraft:match_block','blocks':'jak:'+path,'state':{prop:value}}
    def set_count(path,prop,value,count):
        return {'type':'minecraft:set_count','count':count,'condition':loot_state(path,prop,value)}
    for row in definitions:
        path,look=row['path'],row['look']
        if not row['ready']:
            # A block taken out keeps no drop (its recipe would name an unregistered item).
            (loot/f'{path}.json').unlink(missing_ok=True);continue
        kind,vanilla=look['kind'],look.get('vanilla','')
        drop='altar_turbotabla_vacio' if path=='altar_turbotabla' else path
        entry={'type':'minecraft:item','name':'jak:'+drop}
        functions=[]
        conditions=[]
        if kind=='layers':
            functions.extend(set_count(path,'layers',str(i),i) for i in range(2,9))
        if kind=='slab' or vanilla.endswith('_slab'):
            functions.append(set_count(path,'type','double',2))
        if vanilla.endswith('_door'):
            conditions.append(loot_state(path,'half','lower'))
        if kind in ('plant','leaves','vine'):
            conditions.append({'type':'minecraft:any_of','terms':[
                'minecraft:tool/can_shear','minecraft:tool/can_silk_touch']})
        pool={'rolls':1,'entries':[entry]}
        if conditions:entry['condition']=conditions[0] if len(conditions)==1 else {'type':'minecraft:all_of','terms':conditions}
        functions.append({'type':'minecraft:explosion_decay'})
        entry['modifier']=functions
        table={'type':'minecraft:block','pools':[pool]}
        (loot/(path+'.json')).write_text(json.dumps(table,separators=(',',':')),encoding='utf-8')
        if kind=='leaves':tool='hoe'
        elif kind in ('plant','vine','lily_pad'):tool=None
        elif kind=='pillar' or 'wood' in path or vanilla.startswith(('oak_','spruce_','crimson_')):tool='axe'
        elif kind=='layers' and any(key in path for key in ('sand','snow','mud','arena')):tool='shovel'
        else:tool='pickaxe'
        if tool:tags[tool].append('jak:'+path)
        translations['block.jak.'+path]=row.get('title',path.replace('_',' ').replace('-',' ').capitalize())
    tags["pickaxe"].extend(f"jakmc:vent_{color}" for color in ("red","yellow","blue","green","dark","light"))
    # Optional entries allow the block catalogue to grow without invalidating the vanilla tool tags.
    target=DEST/'data/minecraft/tags/block/mineable'
    target.mkdir(parents=True,exist_ok=True)
    for tool,blocks in tags.items():
        content={'replace':False,'values':[{'id':name,'required':False} for name in blocks]}
        (target/(tool+'.json')).write_text(json.dumps(content,separators=(',',':')),encoding='utf-8')
    languages=DEST/'assets/jak/lang'
    languages.mkdir(parents=True,exist_ok=True)
    # en_us keeps the catalogue titles; es_es takes the Spanish table in nombres_es.py.
    spanish={key:nombre_es(key[len('block.jak.'):],title) for key,title in translations.items()}
    for language,table in (('es_es',spanish),('en_us',translations)):
        (languages/(language+'.json')).write_text(json.dumps(table,ensure_ascii=False),encoding='utf-8')

def seal_track_meshes(catalog):
    for index in (105,106):
        path=catalog/'objects'/f'{index}.json'
        data=json.loads(path.read_text(encoding='utf-8'))
        # The original source uses clockwise faces. The closed construction also
        # includes cut faces inside each curved strip and Minecraft cell.
        surfaces={}
        for cell in data['cells']:
            x,y,z=cell['cell']
            for tri in cell['triangles']:
                for field in ('p','uv','color'):
                    tri[field][1],tri[field][2]=tri[field][2],tri[field][1]
                a,b,c=tri['p']
                up=(b[2]-a[2])*(c[0]-a[0])-(b[0]-a[0])*(c[2]-a[2])
                if up>1e-8:
                    surfaces.setdefault((x,z),[]).append(
                        tuple((p[0]+x,p[1]+y,p[2]+z) for p in tri['p']))
        def interior(point):
            x,y,z=point
            if y<=0:return False
            for a,b,c in surfaces.get((math.floor(x),math.floor(z)),()):
                ax,ay,az=a;bx,by,bz=b;cx,cy,cz=c
                den=(bx-ax)*(cz-az)-(bz-az)*(cx-ax)
                u=((x-ax)*(cz-az)-(z-az)*(cx-ax))/den
                v=((bx-ax)*(z-az)-(bz-az)*(x-ax))/den
                if u>=-1e-6 and v>=-1e-6 and u+v<=1+1e-6 and y<ay+u*(by-ay)+v*(cy-ay)-1e-5:
                    return True
            return False
        for cell in data['cells']:
            outside=[]
            for tri in cell['triangles']:
                a,b,c=tri['p']
                u=[b[i]-a[i] for i in range(3)]
                v=[c[i]-a[i] for i in range(3)]
                normal=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]]
                length=math.sqrt(sum(component*component for component in normal))
                if length<1e-7:continue
                middle=[(a[i]+b[i]+c[i])/3+cell['cell'][i] for i in range(3)]
                if all(interior([middle[i]+side*.0004*normal[i]/length for i in range(3)]) for side in (-1,1)):
                    continue
                outside.append(tri)
            cell['triangles']=outside
        path.write_text(json.dumps(data,separators=(',',':')),encoding='utf-8')

def decorations(catalog):
    # The Rust city uses the arcade cabinet and mirror as authored furniture.
    supported=(86,87,89,90,91,92,94,95,96,97,98,99,100,101,102,103,104)
    (catalog/'decorations.json').write_text(json.dumps(supported),encoding='utf-8')
    items=DEST/'assets/jak/items'
    models=DEST/'assets/jak/models/item'
    # The anchor is the supported base cell, including the actual palm trunk.
    assemblies={88:[0,0,0],93:[3,0,2],105:[0,0,0],106:[3,0,3],107:[1,0,1]}
    roots=json.loads((SOURCE/'wall-roots.json').read_text(encoding='utf-8'))
    for id,root in roots.items():
        assemblies[int(id)]=[math.floor(value+1e-6) for value in root]
    assemblies.update(export_facades(SOURCE/'source-pack/assets/jak/models/block',catalog/'objects'))
    (catalog/'assemblies.json').write_text(json.dumps(assemblies),encoding='utf-8')
    rows=[]
    for id in (*supported,*assemblies):
        root=catalog if id>=108 else SOURCE
        source=json.loads((root/'objects'/f'{id}.json').read_text(encoding='utf-8'))
        path=source['path']
        sprite=min(t['texture'] for cell in source['cells'] for t in cell['triangles'])
        item={'model':{'type':'minecraft:special','base':'jak:item/'+path,
              'model':{'type':'jakmc:prop','id':id}}}
        (items/f'{path}.json').write_text(json.dumps(item),encoding='utf-8')
        model={'gui_light':'side','textures':{'particle':'jak:block/'+sprite},
               'display':{'gui':{'rotation':[25,135,0],'scale':[0.9]*3},
                          'ground':{'translation':[0,3,0],'scale':[0.5]*3},
                          'thirdperson_righthand':{'rotation':[0,90,-30],'scale':[0.4]*3},
                          'firstperson_righthand':{'rotation':[0,45,0],'scale':[0.65]*3}}}
        (models/f'{path}.json').write_text(json.dumps(model),encoding='utf-8')
        rows.append({'path':path,'ready':True,'look':{'kind':'prop'},'title':source['title']})
    return rows

def copy_models():
    source=SOURCE/'source-pack/assets/jak'
    destination=DEST/'assets/jak'
    deferred=set()
    for path in (source/'models/block').glob('*.json'):
        model=json.loads(path.read_text(encoding='utf-8'))
        if any(n < -16 or n > 32 for e in model.get('elements',[]) for n in e['from']+e['to']):
            deferred.add(path.name)
    def skip(directory,names):
        return deferred.intersection(names) if Path(directory)==source/'models/block' else set()
    shutil.copytree(source,destination,dirs_exist_ok=True,ignore=skip)
    for name in deferred:
        (destination/'models/block'/name).unlink(missing_ok=True)
    print(f"Retained {len(deferred)} oversized models in the source export for grid conversion")

def import_blueprints(names):
    catalog=DEST/'data/jakmc/catalog'
    catalog.mkdir(parents=True,exist_ok=True)
    for name in names:
        data=json.loads((SOURCE/f'{name}.json').read_text(encoding='utf-8'))
        (catalog/f'{name}.jbp').write_bytes(packed_blueprint(data))
        print(name,len(data['blocks']),'cells',len(data['props']),'whole props')

# Item transforms of Minecraft's block/block parent, for the Jak blocks drawn as their block.
BLOCK_DISPLAY={'gui':{'rotation':[30,225,0],'scale':[0.625]*3},'ground':{'translation':[0,3,0],'scale':[0.25]*3},
    'fixed':{'scale':[0.5]*3},'thirdperson_righthand':{'rotation':[75,45,0],'translation':[0,2.5,0],'scale':[0.375]*3},
    'firstperson_righthand':{'rotation':[0,45,0],'scale':[0.4]*3},'firstperson_lefthand':{'rotation':[0,225,0],'scale':[0.4]*3}}

def icons():
    # Adrii 06/11: the Jak objects and shaped blocks showed a 16x16 crop of one texture in the inventory.
    # The loose objects (and the older kinds that place them) now draw their own model, as the decorations
    # do (jakmc:prop); a shaped block draws its block model, at the block angle.
    assets=DEST/'assets/jak';catalog=DEST/'data/jakmc/catalog'
    loose=json.loads((catalog/'loose_objects.json').read_text(encoding='utf-8'))
    aliases=json.loads((catalog/'object_aliases.json').read_text(encoding='utf-8'))
    objects=0
    for kind,target in [(int(k),int(k)) for k in loose]+[(int(k),int(v)) for k,v in aliases.items()]:
        source=json.loads((catalog/'objects'/f'{kind}.json').read_text(encoding='utf-8'))
        shape=json.loads((catalog/'objects'/f'{target}.json').read_text(encoding='utf-8'))
        sprite=min(t['texture'] for cell in shape['cells'] for t in cell['triangles'])
        path=source['path']
        item={'model':{'type':'minecraft:special','base':'jak:item/'+path,'model':{'type':'jakmc:prop','id':target}}}
        (assets/'items'/f'{path}.json').write_text(json.dumps(item),encoding='utf-8')
        model={'gui_light':'side','textures':{'particle':'jak:block/'+sprite},
               'display':{'gui':{'rotation':[25,135,0],'scale':[0.9]*3},'ground':{'translation':[0,3,0],'scale':[0.5]*3},
                          'thirdperson_righthand':{'rotation':[0,90,-30],'scale':[0.4]*3},
                          'firstperson_righthand':{'rotation':[0,45,0],'scale':[0.65]*3}}}
        (assets/'models/item'/f'{path}.json').write_text(json.dumps(model),encoding='utf-8')
        objects+=1
    blocks=0
    # Panes and bars keep their flat icon, as Minecraft's glass panes and iron bars (Adrii 06/11: drawn as their
    # block, a thin sheet seen side on, they hardly showed).
    flat={row['path'] for row in json.loads((catalog/'blocks.json').read_text(encoding='utf-8')) if row['look'].get('vanilla') in ('glass_pane','iron_bars')}
    for file in sorted((assets/'items').glob('*.json')):
        if file.stem in flat:continue
        model=json.loads(file.read_text(encoding='utf-8'))['model']
        if model.get('type')!='minecraft:model':continue
        item_model=assets/'models/item'/file.name
        state=assets/'blockstates'/file.name
        if not item_model.is_file() or not state.is_file():continue
        current=json.loads(item_model.read_text(encoding='utf-8'))
        if current.get('parent')!='minecraft:item/generated' or current.get('textures',{}).get('layer0')!='jak:item/'+file.stem:continue
        states=json.loads(state.read_text(encoding='utf-8'))
        if 'variants' in states:
            first=next(iter(states['variants'].values()),None)
        else:
            first=next((part['apply'] for part in states.get('multipart',[])),None)
        if isinstance(first,list):first=first[0] if first else None
        block=first and first.get('model')
        if not block or not block.startswith('jak:block/'):continue
        if not (assets/'models/block'/(block[len('jak:block/'):]+'.json')).is_file():continue
        item_model.write_text(json.dumps({'parent':block,'display':BLOCK_DISPLAY}),encoding='utf-8')
        blocks+=1
    # Adrii 06/11: two blocks (the Haven foliage and burning-can slabs) had no item at all: missing in the hand.
    for loot in sorted((DEST/'data/jak/loot_table/blocks').glob('*.json')):
        path=loot.stem
        if (assets/'items'/f'{path}.json').is_file() or not (assets/'blockstates'/f'{path}.json').is_file():continue
        states=json.loads((assets/'blockstates'/f'{path}.json').read_text(encoding='utf-8'))
        first=next(iter(states['variants'].values()),None) if 'variants' in states else next((part['apply'] for part in states.get('multipart',[])),None)
        if isinstance(first,list):first=first[0] if first else None
        if not first:continue
        (assets/'models/item'/f'{path}.json').write_text(json.dumps({'parent':first['model'],'display':BLOCK_DISPLAY}),encoding='utf-8')
        (assets/'items'/f'{path}.json').write_text(json.dumps({'model':{'type':'minecraft:model','model':'jak:item/'+path}}),encoding='utf-8')
        blocks+=1
    print('Item icons:',objects,'objects drawn as their model,',blocks,'blocks as their block model')

# Adrii 06/11: every piece of Jak decoration is made. In the stonecutter: one block of the place's material,
# the piece chosen from the list (the crafting table cannot offer a choice for the same materials).
PLACES=(('spargus',('sp_','sp-','spargus','sparg_','muro_spargus','objeto_spargus','arena_','cumbrera_roja')),
        ('palace',('palacio',)),
        ('haven',('hv_','hv-','haven','barrio','pieza_haven','objeto_haven','objeto_villarrefugio','suelo_villarrefugio',
                  'muro_villarrefugio','muro_haven','krimson','bar_','luz_','cumbrera_chapa')),
        ('sandover',('v1_','v1-')),
        ('snow',('sn_','sn-','snow','nieve','montana','objeto_montana','muro_montana','cumbrera_nevada')),
        ('swamp',('sw_','sw-','swamp','pantano','objeto_pantano','muro_pantano','cumbrera_pantano')),
        ('wasteland',('wasteland','desert')))
STONE={'spargus':'minecraft:sandstone','palace':'minecraft:quartz_block','haven':'minecraft:stone_bricks','sandover':'minecraft:cobblestone',
       'snow':'minecraft:stone','swamp':'minecraft:mossy_cobblestone','wasteland':'minecraft:red_sandstone','other':'minecraft:stone'}
WOOD={'spargus':'jak:spargus_planks','palace':'minecraft:dark_oak_planks','haven':'minecraft:oak_planks','sandover':'minecraft:oak_planks',
      'snow':'minecraft:spruce_planks','swamp':'minecraft:mangrove_planks','wasteland':'minecraft:jungle_planks','other':'minecraft:oak_planks'}
ROOF={'spargus':'minecraft:iron_ingot','palace':'minecraft:bricks','haven':'minecraft:bricks','sandover':'minecraft:hay_block',
      'snow':'minecraft:spruce_planks','swamp':'minecraft:hay_block','wasteland':'minecraft:iron_ingot','other':'minecraft:bricks'}
# By what the piece is made of, from its name (Spanish and the original texture names), first match.
MATERIALS=((('glass','vidrio','window','ventana','cristal'),'minecraft:glass'),
           (('light','luz','lamp','farol','lantern','neon','glow','bombilla'),'minecraft:glowstone'),
           (('straw','paja','hay','thatch'),'minecraft:hay_block'),
           (('rope','cuerda','string'),'minecraft:string'),
           (('cloth','tela','banner','flag','bandera','estandarte','toldo','awning','sail','vela','curtain','cortina','carpet','alfombra','rug'),'minecraft:white_wool'),
           (('precursor','warpgate','sculpt','sage','brass','bronce','copper','cobre','gold','oro'),'minecraft:copper_ingot'),
           (('metal','iron','hierro','steel','acero','pipe','tubo','tuberia','rail','grate','rejilla','reja','siding','chapa','bars','barrotes','gear',
             'blindaje','terminal','insignia','desague','sign','cartel','rotulo','propaganda'),'minecraft:iron_ingot'),
           (('palmtree','palm','palmera','trunk','tronco','log'),'minecraft:jungle_log'),
           (('wood','madera','plank','tabla','tablon','board','barril','barrel','crate','caja','box','post','poste','beam','viga','fence','valla',
             'door','puerta','porton','bench','banco','mesa','table','taburete','stool','shelf','andamio','scaffold','entramado','larguero',
             'puntal','mensula','ladder','marco','frame','urna','vase','jarr','pot','bidon','trofeo','botell',
             'jarra','bandeja','juego','espejo','mostrador','servicio','coleccion'),'WOOD'),
           (('roof','tejado','tejadillo','teja','cumbrera','tile','alero','azotea','cupula'),'ROOF'),
           (('brick','ladrillo'),'minecraft:bricks'),
           (('snow','nieve'),'minecraft:snow_block'),
           (('rice','arroz'),'minecraft:dirt'),
           (('ice','hielo'),'minecraft:ice'),
           (('mud','barro','lodo'),'minecraft:mud'),
           (('sand','arena','dune'),'minecraft:sand'),
           (('dirt','tierra','soil'),'minecraft:dirt'),
           (('leaf','leaves','hoja','frond','planta','plant','grass','hierba','hierbajos','bush','arbusto','mata','flower','flor','vine','enredadera','moss','musgo','canes'),'PLANT'))

def place(path):
    for name,prefixes in PLACES:
        if path.startswith(prefixes) or any(p.strip('_-') in path for p in prefixes if len(p)>4):return name
    return 'other'

def recipe_material(path,kind):
    if kind in ('plant','vine','lily_pad'):return 'minecraft:short_grass'
    words=path.lower()
    for keys,material in MATERIALS:
        if any(key in words for key in keys):
            where=place(path)
            if material=='WOOD':return WOOD[where]
            if material=='ROOF':return ROOF[where]
            if material=='PLANT':return 'minecraft:oak_leaves' if kind in ('cube','leaves') else 'minecraft:short_grass'
            return material
    return 'minecraft:oak_leaves' if kind=='leaves' and 'hv_city' not in path.lower() else STONE[place(path)]

def recipes(definitions):
    """A stonecutting recipe for every Jak block the game registers (its loot table); the older loose-object items place one of these."""
    target=DEST/'data/jak/recipe'
    if target.exists():shutil.rmtree(target)
    target.mkdir(parents=True)
    kinds={row['path']:(row['look']['kind'],row['look'].get('vanilla','')) for row in definitions}
    made=set(json.loads(r.read_text(encoding='utf-8'))['result']['id'] for r in (ROOT/'src/main/resources/data/jakmc/recipe').glob('*.json'))
    loot=DEST/'data/jak/loot_table/blocks'
    ids={p.stem for p in loot.glob('*.json')}
    ids.discard('altar_turbotabla')
    written=0
    for path in sorted(ids):
        if 'jak:'+path in made or path=='palm_log':continue
        kind,vanilla=kinds.get(path,('object',''))
        material=recipe_material(path,kind)
        if material=='jak:'+path:continue
        count=2 if kind in ('slab','layers') or vanilla.endswith(('_slab','_pane','_bars','_rod','_wall')) or vanilla in ('glass_pane','iron_bars','lightning_rod','cobblestone_wall','leaf_litter') else 1
        recipe={'type':'minecraft:stonecutting','ingredient':material,'result':{'id':'jak:'+path,'count':count}}
        (target/(path+'.json')).write_text(json.dumps(recipe,separators=(',',':')),encoding='utf-8')
        written+=1
    print('Stonecutting recipes:',written)

def main():
    if not (SOURCE/'objects/104.json').is_file():raise RuntimeError('Source export is incomplete; run tools/content-exporter first')
    DEST.mkdir(parents=True,exist_ok=True)
    copy_models()
    fit_architecture()
    catalog=DEST/'data/jakmc/catalog';catalog.mkdir(parents=True,exist_ok=True)
    definitions=json.loads((SOURCE/'blocks.json').read_text(encoding='utf-8'))
    configure_materials(definitions)
    (catalog/'blocks.json').write_text(json.dumps(definitions,separators=(',',':')),encoding='utf-8')
    shutil.copytree(SOURCE/'objects',catalog/'objects',dirs_exist_ok=True)
    material_resources(definitions+decorations(catalog))
    seal_track_meshes(catalog)
    import_blueprints(('haven','spargus','palace')+SITES)
    import import_facade_parts;import_facade_parts.export(SOURCE,DEST)
    icons()
    recipes(definitions)
    receipts=[]
    for path in sorted(DEST.rglob('*')):
        if path.is_file():receipts.append(dict(path=str(path.relative_to(DEST)).replace('\\','/'),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),size=path.stat().st_size))
    (ROOT/'.local/content-resource-receipt.json').write_text(json.dumps(receipts,indent=2),encoding='utf-8')
    print('Imported',len(receipts),'private resource files. Source geometry remains a build-time export, not a runtime dependency.')

if __name__=='__main__':
    import sys
    if sys.argv[1:]==['--sites-only']:
        import_blueprints(SITES)
    elif sys.argv[1:]==['--icons']:
        icons()
    elif sys.argv[1:]==['--recipes']:
        recipes(json.loads((DEST/'data/jakmc/catalog/blocks.json').read_text(encoding='utf-8')))
    else:
        main()
