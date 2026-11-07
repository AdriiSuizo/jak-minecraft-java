"""Installer boundary checks; uses a temporary launcher with synthetic accounts."""
import importlib.util, json, tempfile, unittest, zipfile
from pathlib import Path

spec=importlib.util.spec_from_file_location('jak_installer',Path(__file__).with_name('install.py'))
installer=importlib.util.module_from_spec(spec);spec.loader.exec_module(installer)

class InstallerChecks(unittest.TestCase):
    def test_slimming_keeps_named_models_and_never_enters_a_junction(self):
        with tempfile.TemporaryDirectory() as temp:
            payload=Path(temp)/'payload';payload.mkdir();(payload/'model-tokens.txt').write_text('sidekick\ncitizen-\n')
            old=installer.PAYLOAD;installer.PAYLOAD=payload
            try:
                root=Path(temp)/'export';level=root/'levels/a';level.mkdir(parents=True);(root/'collision').mkdir()
                names=['sidekick-lod0.glb','citizen-fat-lod0.glb','sage-lod0.glb','sidekick-lod1.glb','a-background.glb','flutflut-lod0.glb']
                for name in names:(level/name).write_bytes(b'x')
                installer.slim_models(root,'jak1')
                self.assertEqual(sorted(p.name for p in level.iterdir()),['a-background.glb','citizen-fat-lod0.glb','flutflut-lod0.glb','sidekick-lod0.glb'])
                self.assertFalse((root/'collision').exists())
                installer.slim_converted(root)
                self.assertFalse((level/'a-background.glb').exists())
            finally:installer.PAYLOAD=old
            mine=Path(temp)/'owned';theirs=Path(temp)/'opengoal';theirs.mkdir();(theirs/'keep').write_text('x');(mine/'jak2').mkdir(parents=True)
            installer.junction(mine/'jak1',theirs)
            self.assertEqual(installer.owned_games(mine),[mine/'jak2'])
            installer.forget_extraction(mine)
            self.assertTrue((theirs/'keep').is_file());self.assertFalse((mine/'jak1').exists())

    def test_active_and_parent_selection(self):
        with tempfile.TemporaryDirectory() as temp:
            parent=Path(temp)/'OpenGOAL';active=parent/'active'
            for game in ('jak1','jak2','jak3'):(active/game/'data').mkdir(parents=True)
            self.assertEqual(installer.normalize_opengoal(parent),active.resolve())
            self.assertEqual(installer.normalize_opengoal(active/'jak2'),active.resolve())
            self.assertIsNone(installer.normalize_opengoal(Path(temp)/'missing'))

    def test_profile_merge_preserves_accounts_worlds_and_other_profiles(self):
        with tempfile.TemporaryDirectory() as temp:
            mc=Path(temp)/'Minecraft con espacios ñ';instance=mc/'instances/jakmod'
            base=mc/'versions/26.3/26.3.json';base.parent.mkdir(parents=True);base.write_text('{}')
            sentinel={'profiles':{'personal':{'name':'Never change'}},'authenticationDatabase':{'synthetic':'keep'},'settings':{'other':True}}
            store=mc/'launcher_profiles.json';store.write_text(json.dumps(sentinel))
            save=mc/'saves/Real World/level.dat';save.parent.mkdir(parents=True);save.write_bytes(b'world-sentinel')
            def download(url,dest,*args):
                if url.endswith('/profile/json'):
                    installer.write_json(dest,{'id':'fabric','inheritsFrom':'26.3','libraries':[], 'arguments':{'jvm':[]}})
                else:installer.atomic(dest,b'fixture')
            manifest={'minecraft':'26.3','loader':'0.20.1','fabric_api':'0.165.0+26.3','fabric_api_sha512':'fixture'}
            self.assertEqual(installer.install_launcher(mc,instance,manifest,'es',download),'26.3.jakmod')
            data=json.loads(store.read_text())
            for key in ('authenticationDatabase','settings'):self.assertEqual(data[key],sentinel[key])
            self.assertEqual(data['profiles']['personal'],sentinel['profiles']['personal'])
            self.assertEqual(data['profiles']['jakmod']['lastVersionId'],'26.3.jakmod')
            self.assertEqual(save.read_bytes(),b'world-sentinel')
            backups=list((instance/'jakmc/backups').glob('*.json'));self.assertEqual(len(backups),1)
            self.assertEqual(json.loads(backups[0].read_text()),sentinel)
            self.assertIn('es_es',(instance/'options.txt').read_text())
            installer.install_launcher(mc,instance,manifest,'en',download)
            self.assertEqual(len(json.loads(store.read_text())['profiles']),2)
            self.assertIn('es_es',(instance/'options.txt').read_text(),'updates retain player settings')

    def test_resource_merge_has_no_duplicate_zip_members(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);code=root/'code.jar';resources=root/'resources';resources.mkdir()
            (resources/'same').write_bytes(b'owned-local-conversion')
            with zipfile.ZipFile(code,'w') as out:
                out.writestr('code.class',b'code');out.writestr('same',b'old')
            dest=root/'mods/final.jar';installer.merge_mod(code,resources,dest)
            with zipfile.ZipFile(dest) as z:
                self.assertEqual(z.namelist().count('same'),1)
                self.assertEqual(z.read('same'),b'owned-local-conversion')
                self.assertEqual(z.read('code.class'),b'code')

    def test_bad_download_does_not_replace_existing_file(self):
        with tempfile.TemporaryDirectory() as temp:
            p=Path(temp)/'keep';p.write_bytes(b'sentinel')
            # A data URL avoids external network traffic in this boundary test.
            with self.assertRaises(ValueError):installer.fetch('data:text/plain,wrong',p,'0'*64)
            self.assertEqual(p.read_bytes(),b'sentinel')

if __name__=='__main__':unittest.main()
