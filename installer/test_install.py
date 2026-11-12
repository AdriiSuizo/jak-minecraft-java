"""Installer boundary checks; uses a temporary launcher with synthetic accounts."""
import importlib.util, json, tempfile, unittest, urllib.error, zipfile
from pathlib import Path
from unittest.mock import patch

spec=importlib.util.spec_from_file_location('jak_installer',Path(__file__).with_name('install.py'))
installer=importlib.util.module_from_spec(spec);spec.loader.exec_module(installer)

class InstallerChecks(unittest.TestCase):
    def test_slimming_keeps_named_models_and_never_enters_a_junction(self):
        with tempfile.TemporaryDirectory() as temp:
            payload=Path(temp)/'payload';payload.mkdir();(payload/'model-tokens.txt').write_text('sidekick\ncitizen-\nwhole-lod0\n')
            old=installer.PAYLOAD;installer.PAYLOAD=payload
            try:
                root=Path(temp)/'export';level=root/'levels/a';level.mkdir(parents=True);(root/'collision').mkdir()
                names=['sidekick-lod0.glb','citizen-fat-lod0.glb','whole-lod0.glb','sage-lod0.glb','sidekick-lod1.glb','a-background.glb','flutflut-lod0.glb']
                for name in names:(level/name).write_bytes(b'x')
                installer.slim_models(root,'jak1')
                self.assertEqual(sorted(p.name for p in level.iterdir()),['a-background.glb','citizen-fat-lod0.glb','flutflut-lod0.glb','sidekick-lod0.glb','whole-lod0.glb'])
                self.assertTrue((root/'collision').exists())
                installer.slim_converted(root)
                self.assertFalse((root/'collision').exists())
                self.assertFalse((level/'a-background.glb').exists())
            finally:installer.PAYLOAD=old
            mine=Path(temp)/'owned';theirs=Path(temp)/'opengoal';theirs.mkdir();(theirs/'keep').write_text('x');(mine/'jak2').mkdir(parents=True)
            installer.junction(mine/'jak1',theirs)
            self.assertEqual(installer.owned_games(mine),[mine/'jak2'])
            installer.junction(mine/'jak2/audio',theirs)
            installer.slim_converted(mine/'jak2')
            self.assertTrue((theirs/'keep').is_file())
            self.assertTrue((mine/'jak2/audio').is_junction())

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
            data=json.loads(store.read_text(encoding='utf-8'))
            for key in ('authenticationDatabase','settings'):self.assertEqual(data[key],sentinel[key])
            self.assertEqual(data['profiles']['personal'],sentinel['profiles']['personal'])
            self.assertEqual(data['profiles']['jakmod']['lastVersionId'],'26.3.jakmod')
            self.assertEqual(save.read_bytes(),b'world-sentinel')
            backups=list((instance/'jakmc/backups').glob('*.json'));self.assertEqual(len(backups),1)
            self.assertEqual(json.loads(backups[0].read_text()),sentinel)
            self.assertIn('es_es',(instance/'options.txt').read_text())
            data['profiles']['jakmod'].update(name='My Jak',javaArgs='-Xmx6G -Dcustom=true',icon='Diamond')
            installer.write_json(store,data)
            installer.install_launcher(mc,instance,manifest,'en',download)
            updated=json.loads(store.read_text(encoding='utf-8'))['profiles']
            self.assertEqual(len(updated),2)
            self.assertEqual(updated['jakmod'],data['profiles']['jakmod'])
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

    def test_showcase_installs_once_and_preserves_the_players_copy(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);stage=root/'staging';stage.mkdir()
            world=root/'world.zip'
            with zipfile.ZipFile(world,'w') as archive:
                archive.writestr('level.dat',b'world fixture')
                archive.writestr('dimensions/minecraft/overworld/region/r.0.0.mca',b'fixture region')
            target=root/'saves'/installer.SHOWCASE
            tx=installer.InstallationFiles(stage,root/'backups')
            tx.add_showcase(root,world)
            self.assertFalse(target.exists())
            tx.commit('es')
            self.assertEqual((target/'level.dat').read_bytes(),b'world fixture')
            self.assertTrue((target/'dimensions/minecraft/overworld/region/r.0.0.mca').is_file())
            (target/'level.dat').write_bytes(b'player changes')
            update=installer.InstallationFiles(stage,root/'backups')
            update.add_showcase(root,world)
            update.commit('en')
            self.assertEqual((target/'level.dat').read_bytes(),b'player changes')

    def test_showcase_does_not_invalidate_the_asset_conversion_cache(self):
        manifest={'files':{'jak-content-exporter.exe':'exporter','jakmc-code.jar':'mod'}}
        stamp=installer.conversion_stamp(manifest)
        manifest['files'][installer.SHOWCASE_ARCHIVE]='first world'
        self.assertEqual(installer.conversion_stamp(manifest),stamp)
        manifest['files'][installer.SHOWCASE_ARCHIVE]='new world'
        self.assertEqual(installer.conversion_stamp(manifest),stamp)

    def test_legacy_showcase_is_used_without_adding_a_second_map(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);stage=root/'staging';stage.mkdir()
            world=root/'saves/Jak Showcase/level.dat'
            world.parent.mkdir(parents=True);world.write_bytes(b'player world')
            tx=installer.InstallationFiles(stage,root/'backups')
            self.assertEqual(tx.add_showcase(root,root/'unused.zip'),'Jak Showcase')
            tx.commit('es')
            self.assertEqual(world.read_bytes(),b'player world')
            self.assertEqual([p.name for p in (root/'saves').iterdir()],['Jak Showcase'])

    def test_showcase_is_rolled_back_when_launcher_commit_fails(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);stage=root/'staging';stage.mkdir()
            world=root/'world.zip'
            with zipfile.ZipFile(world,'w') as archive:archive.writestr('level.dat',b'world fixture')
            tx=installer.InstallationFiles(stage,root/'backups')
            tx.add_showcase(root,world)
            store=root/'launcher_profiles.json'
            store.write_bytes(b'concurrent change')
            tx.guards[store]=None
            tx.write(store,b'installer profile')
            with self.assertRaises(installer.InstallError):tx.commit('es')
            self.assertFalse((root/'saves'/installer.SHOWCASE).exists())
            self.assertEqual(store.read_bytes(),b'concurrent change')

    def test_bad_download_does_not_replace_existing_file(self):
        with tempfile.TemporaryDirectory() as temp:
            p=Path(temp)/'keep';p.write_bytes(b'sentinel')
            # A data URL avoids external network traffic in this boundary test.
            with self.assertRaises(installer.InstallError):installer.fetch('data:text/plain,wrong',p,'0'*64)
            self.assertEqual(p.read_bytes(),b'sentinel')
            self.assertEqual(list(Path(temp).iterdir()),[p])

    def test_failed_commit_rolls_back_new_and_replaced_files(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp); staging=root/'staging';staging.mkdir()
            mod=root/'mods/jakmc.jar';mod.parent.mkdir();mod.write_bytes(b'previous jar')
            new=root/'versions/fabric.json'
            store=root/'launcher_profiles.json';store.write_bytes(b'launcher data')
            tx=installer.InstallationFiles(staging,root/'backups')
            tx.write(mod,b'updated jar')
            tx.write(new,b'fabric')
            tx.write(store,b'new launcher data')
            replace=installer.os.replace
            def fail_at_store(source,target):
                if target==store: raise PermissionError('launcher file locked')
                replace(source,target)
            with patch.object(installer.os,'replace',side_effect=fail_at_store):
                with self.assertRaises(PermissionError):tx.commit('es')
            self.assertEqual(mod.read_bytes(),b'previous jar')
            self.assertFalse(new.exists())
            self.assertEqual(store.read_bytes(),b'launcher data')

    def test_launcher_guard_does_not_overwrite_a_concurrent_edit(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);staging=root/'staging';staging.mkdir()
            store=root/'launcher_profiles.json';store.write_bytes(b'new launcher settings')
            tx=installer.InstallationFiles(staging,root/'backups')
            tx.guards[store]=b'old launcher settings'
            tx.write(store,b'installation')
            with self.assertRaises(installer.InstallError):tx.commit('en')
            self.assertEqual(store.read_bytes(),b'new launcher settings')

    def test_only_duplicate_managed_mod_ids_are_archived(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);staging=root/'staging';staging.mkdir()
            mods=root/'mods';mods.mkdir()
            files={'old-renamed.jar':'sodium','current.jar':'sodium','another.jar':'another'}
            for name,mod_id in files.items():
                with zipfile.ZipFile(mods/name,'w') as jar:
                    jar.writestr('fabric.mod.json',json.dumps({'id':mod_id}))
            (mods/'damaged.jar').write_bytes(b'unrelated unknown format')
            tx=installer.InstallationFiles(staging,root/'backups')
            installer.archive_old_mods(root,{'sodium':mods/'current.jar'},tx)
            tx.commit('en')
            self.assertEqual({p.name for p in mods.iterdir()},{'current.jar','another.jar','damaged.jar'})
            self.assertEqual(len(list((root/'backups').glob('old-renamed-*.jar'))),1)

    def test_conversion_cache_tracks_source_and_validates_resources(self):
        with tempfile.TemporaryDirectory() as temp:
            root=Path(temp);vanilla=root/'vanilla.jar';vanilla.write_bytes(b'a')
            manifest={'files':{'jakmc-code.jar':'one'},'converters':{'import.py':'two'}}
            gen=root/'first'
            key=installer.asset_identity(gen,vanilla,manifest)
            self.assertNotEqual(key,installer.asset_identity(root/'second',vanilla,manifest))
            manifest['files']['jakmc-code.jar']='new code'
            self.assertEqual(key,installer.asset_identity(gen,vanilla,manifest))
            manifest['converters']['import.py']='new conversion'
            self.assertNotEqual(key,installer.asset_identity(gen,vanilla,manifest))
            for game,required in installer.REQUIRED.items():
                model=installer.export_root(gen/'owned/active',game)/required[0]
                installer.atomic(model,b'model')
            resources=gen/'src/generated/resources'
            for name in installer.REQUIRED_RESOURCES:installer.atomic(resources/name,b'test')
            installer.write_json(gen/'complete.json',{'identity':key,'count':len(installer.REQUIRED_RESOURCES),
                'required':{name:4 for name in installer.REQUIRED_RESOURCES}})
            self.assertIsNotNone(installer.cached_assets(gen,key))
            (resources/installer.REQUIRED_RESOURCES[0]).unlink()
            self.assertIsNone(installer.cached_assets(gen,key))

    def test_comment_report_contains_no_local_paths_or_error_urls(self):
        manifest={'minecraft':'26.3','mod_version':'0.1.0-dev'}
        failure=PermissionError(13,'access denied',r'C:\Users\Alice\secret')
        _,report=installer.support_report(failure,'commit',manifest,'es')
        self.assertIn('Actualizar el perfil',report)
        self.assertIn('26.3',report)
        self.assertNotIn('Alice',report)
        http=urllib.error.HTTPError('https://example.org/?key=secret',503,'unavailable',{},None)
        _,report=installer.support_report(http,'dependencies',manifest,'en')
        self.assertIn('HTTP 503',report)
        self.assertNotIn('secret',report)
        self.assertEqual(installer.youtube_link('https://www.youtube.com/@AdriiSuizo'),'https://www.youtube.com/@AdriiSuizo')
        self.assertIsNone(installer.youtube_link('https://youtube.com.example.org/video'))

if __name__=='__main__':unittest.main()
