"""Build private Minecraft resources from the source export. No retail assets are tracked."""
from pathlib import Path
import gzip,hashlib,json,math,shutil,struct

ROOT=Path(__file__).resolve().parents[1]
SOURCE=ROOT/'.local/content'
DEST=ROOT/'src/generated/resources'

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
    model=SOURCE/'source-pack/assets/jak/models/block'/f'{path}.json'
    elements=json.loads(model.read_text(encoding='utf-8')).get('elements',[])
    if not elements or any('rotation' in e or any(v<0 or v>16 for v in e['from']+e['to']) for e in elements):
        return None
    return [e['from']+e['to'] for e in elements]

def configure_materials(definitions):
    for row in definitions:
        kind,vanilla=row['look']['kind'],row['look'].get('vanilla','')
        if vanilla in ('carved_pumpkin','stone','sea_lantern'):
            boxes=authored_boxes(row['path'])
            if boxes is not None:
                kind='authored' if vanilla=='carved_pumpkin' else 'static_authored'
                row['look']={'kind':kind,'boxes':boxes,'vanilla':vanilla}
        row['ready']=not row['path'].startswith('objeto_') and (kind!='native' or vanilla in NATIVE_MATERIALS)

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
        if not row['ready']:continue
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
    # Optional entries allow the block catalogue to grow without invalidating the vanilla tool tags.
    target=DEST/'data/minecraft/tags/block/mineable'
    target.mkdir(parents=True,exist_ok=True)
    for tool,blocks in tags.items():
        content={'replace':False,'values':[{'id':name,'required':False} for name in blocks]}
        (target/(tool+'.json')).write_text(json.dumps(content,separators=(',',':')),encoding='utf-8')
    languages=DEST/'assets/jak/lang'
    languages.mkdir(parents=True,exist_ok=True)
    for language in ('es_es','en_us'):
        (languages/(language+'.json')).write_text(json.dumps(translations,ensure_ascii=False),encoding='utf-8')

def decorations(catalog):
    # The minigame and mirror need their original runtime behavior.
    supported=(86,87,89,90,91,92,94,95,96,97,98,99,100,103,104)
    (catalog/'decorations.json').write_text(json.dumps(supported),encoding='utf-8')
    items=DEST/'assets/jak/items'
    models=DEST/'assets/jak/models/item'
    # The anchor is the supported base cell, including the actual palm trunk.
    assemblies={88:[0,0,0],93:[3,0,2]}
    roots=json.loads((SOURCE/'wall-roots.json').read_text(encoding='utf-8'))
    for id,root in roots.items():
        assemblies[int(id)]=[math.floor(value+1e-6) for value in root]
    (catalog/'assemblies.json').write_text(json.dumps(assemblies),encoding='utf-8')
    rows=[]
    for id in (*supported,*assemblies):
        source=json.loads((SOURCE/'objects'/f'{id}.json').read_text(encoding='utf-8'))
        path=source['path']
        sprite=source['cells'][0]['triangles'][0]['texture']
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

def main():
    if not (SOURCE/'objects/104.json').is_file():raise RuntimeError('Source export is incomplete; run tools/content-exporter first')
    DEST.mkdir(parents=True,exist_ok=True)
    copy_models()
    catalog=DEST/'data/jakmc/catalog';catalog.mkdir(parents=True,exist_ok=True)
    definitions=json.loads((SOURCE/'blocks.json').read_text(encoding='utf-8'))
    configure_materials(definitions)
    (catalog/'blocks.json').write_text(json.dumps(definitions,separators=(',',':')),encoding='utf-8')
    material_resources(definitions+decorations(catalog))
    shutil.copytree(SOURCE/'objects',catalog/'objects',dirs_exist_ok=True)
    for name in ['haven','spargus','palace']:
        data=json.loads((SOURCE/f'{name}.json').read_text(encoding='utf-8'))
        (catalog/f'{name}.jbp').write_bytes(packed_blueprint(data))
        print(name,len(data['blocks']),'cells',len(data['props']),'whole props')
    receipts=[]
    for path in sorted(DEST.rglob('*')):
        if path.is_file():receipts.append(dict(path=str(path.relative_to(DEST)).replace('\\','/'),sha256=hashlib.sha256(path.read_bytes()).hexdigest(),size=path.stat().st_size))
    (ROOT/'.local/content-resource-receipt.json').write_text(json.dumps(receipts,indent=2),encoding='utf-8')
    print('Imported',len(receipts),'private resource files. Source geometry remains a build-time export, not a runtime dependency.')

if __name__=='__main__':main()
