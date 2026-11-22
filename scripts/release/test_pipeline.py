"""Fail-closed release controls; tests never sign, create tags or mutate registries/Releases."""
import contextlib
import copy
import importlib.util
import io
import json
import os
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch
import zipfile

spec = importlib.util.spec_from_file_location('pipeline', Path(__file__).with_name('pipeline.py'))
p = importlib.util.module_from_spec(spec)
spec.loader.exec_module(p)
REPO_ROOT = Path(__file__).resolve().parents[2]


class Controls(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self.tmp.cleanup)
        self.root = Path(self.tmp.name)
        self.previous = Path.cwd()
        os.chdir(self.root)
        self.addCleanup(os.chdir, self.previous)
        self.value = '0.2.0'
        self.bundle = self.root / 'bundle'
        self.bundle.mkdir()
        Path('Cargo.toml').write_text('[package]\nname="irig106-ch10-reader"\nversion="0.2.0"\n[[bin]]\nname="ch10r"\npath="src/main.rs"\n')
        (self.bundle / f'{p.NAME}-{self.value}.crate').write_bytes(b'crate')
        (self.bundle / 'sbom.cdx.json').write_text('{}')
        self.zip()
        p.assemble(self.bundle, self.value)
        self.env = patch.dict(os.environ, {'GITHUB_OUTPUT': '', 'GITHUB_STEP_SUMMARY': '',
            'GITHUB_EVENT_NAME': 'push', 'GITHUB_REF_TYPE': 'tag', 'GITHUB_REF_NAME': 'v0.2.0',
            'GITHUB_REF': 'refs/tags/v0.2.0', 'GITHUB_SHA': 'a'*40, 'RELEASE_COMMIT': 'a'*40})
        self.env.start()
        self.addCleanup(self.env.stop)

    def zip(self, members=None):
        with zipfile.ZipFile(self.bundle / p.archive_name(self.value), 'w') as archive:
            for name, raw in (members or {'ch10r.exe': b'MZexecutable', 'LICENSE': b'license', 'README.md': b'readme'}).items():
                archive.writestr(name, raw)

    def test_exact_bundle(self):
        self.assertEqual(set(p.validate_bundle(self.bundle, self.value)), p.expected_files(self.value))

    def test_checksum_rejects_tampering_missing_extra_and_symlinks(self):
        file = self.bundle / 'sbom.cdx.json'
        file.write_bytes(b'corrupt')
        with self.assertRaises(ValueError): p.validate_bundle(self.bundle, self.value)
        file.write_text('{}')
        file.unlink()
        with self.assertRaises(ValueError): p.validate_bundle(self.bundle, self.value)
        outside = self.root / 'outside'; outside.write_text('{}')
        file.symlink_to(outside)
        with self.assertRaises(ValueError): p.validate_bundle(self.bundle, self.value)
        file.unlink(); file.write_text('{}')
        (self.bundle / 'extra').write_text('bad')
        with self.assertRaises(ValueError): p.validate_bundle(self.bundle, self.value)

    def test_checksum_manifest_rejects_unsafe_duplicate_and_missing_entries(self):
        manifest = self.bundle / 'SHA256SUMS'; original = manifest.read_text()
        for text in [original + original.splitlines()[0] + '\n', original.replace('sbom.cdx.json', '../escape'),
                     '\n'.join(original.splitlines()[1:]), original+'\n', original.replace('  ', ' *')]:
            with self.subTest(text=text):
                manifest.write_text(text)
                with self.assertRaises(ValueError): p.validate_bundle(self.bundle, self.value)

    def test_archive_rejects_traversal_extra_missing_or_non_executable(self):
        for members in [{'../ch10r.exe': b'MZ'}, {'ch10r.exe': b'not PE', 'LICENSE': b'x', 'README.md': b'x'},
                        {'ch10r.exe': b'MZ', 'LICENSE': b'x'},
                        {'ch10r.exe': b'MZ', 'LICENSE': b'x', 'README.md': b'x', 'other.exe': b'MZ'}]:
            self.zip(members)
            with self.assertRaises(ValueError): p.check_archive(self.bundle / p.archive_name(self.value))

    def test_all_binary_targets_are_selected(self):
        metadata = {'packages': [{'name': p.NAME, 'targets': [{'name': 'ch10r', 'kind':['bin']},
                     {'name': 'other', 'kind':['bin']}, {'name':'lib','kind':['lib']}]}]}
        with patch.object(p, 'run', return_value=json.dumps(metadata)):
            self.assertEqual(p.binary_names(), ['ch10r','other'])

    def test_version_no_workspace_or_internal_helper_shortcut(self):
        self.assertEqual(p.version(), self.value)
        with open('Cargo.toml','a') as stream: stream.write('\n[workspace]\nmembers=[]\n')
        with self.assertRaises(ValueError): p.version()

    def test_api_annotation_and_peeling(self):
        with patch.object(p, 'gh_api', return_value={'object':{'type':'commit','sha':'a'*40}}):
            with self.assertRaises(ValueError): p.tag_commit('v0.2.0')
        with patch.object(p, 'gh_api', side_effect=[{'object':{'type':'tag','sha':'b'*40}},
                                                  {'object':{'type':'commit','sha':'a'*40}}]):
            self.assertEqual(p.tag_commit('v0.2.0'), 'a'*40)

    def test_intended_ci_exact_identity_and_all_pages(self):
        good={'head_sha':'a'*40,'path':'.github/workflows/windows-ci.yml','event':'push','head_branch':'main','conclusion':'success'}
        self.assertTrue(p.ci_succeeded([{'workflow_runs':[]},{'workflow_runs':[good]}], 'a'*40))
        for key,value in [('path','other.yml'),('head_sha','b'*40),('event','pull_request'),('head_branch','topic'),('conclusion','failure')]:
            self.assertFalse(p.ci_succeeded([{'workflow_runs':[dict(good,**{key:value})]}], 'a'*40))

    def gate(self, *, registry=True, ci=True, ancestor=True, api_commit='a'*40, notes=True, tag='v0.2.0'):
        def run(*args, **kwargs):
            if args[0]=='gh':
                return json.dumps([{'workflow_runs':[{'head_sha':'a'*40,'path':'.github/workflows/windows-ci.yml',
                    'event':'push','head_branch':'main','conclusion':'success' if ci else 'failure'}]}])
            if args[1]=='cat-file': return 'tag'
            return 'a'*40
        with patch.object(p,'run',side_effect=run), patch.object(p,'tag_commit',return_value=api_commit), \
             patch.object(p,'http_get',return_value=b'{"crate":{"trustpub_only":true}}' if registry else None), \
             patch.object(p,'release_record',side_effect=None if notes else FileNotFoundError('notes')), \
             patch.object(p.subprocess,'run',return_value=subprocess.CompletedProcess([],0 if ancestor else 1)), \
             patch.object(p,'output') as output, patch.dict(os.environ,{'GITHUB_REF_NAME':tag}), \
             contextlib.redirect_stdout(io.StringIO()):
            p.gate()
            return output.call_args.kwargs

    def test_complete_tag_gate(self):
        self.assertEqual(self.gate()['publish'],'true')

    def test_gate_refuses_bootstrap_or_bad_evidence(self):
        for changes in [{'registry':False},{'ci':False},{'ancestor':False},{'api_commit':'b'*40},{'notes':False},{'tag':'v0.3.0'}]:
            with self.subTest(changes=changes), self.assertRaises(ValueError): self.gate(**changes)

    def test_dry_runs_are_unsigned_even_manual_from_tag(self):
        for event in ['pull_request','workflow_dispatch']:
            with patch.dict(os.environ,{'GITHUB_EVENT_NAME':event}):
                self.assertEqual(self.gate(registry=False,notes=False)['publish'],'false')

    def test_script_guard_rejects_dry_runs_wrong_tag_or_commit(self):
        for env in [{'GITHUB_EVENT_NAME':'pull_request'},{'GITHUB_EVENT_NAME':'workflow_dispatch'},
                    {'GITHUB_REF_TYPE':'branch'},{'GITHUB_REF':'refs/tags/v0.3.0'},{'RELEASE_COMMIT':'b'*40}]:
            with patch.dict(os.environ,env), patch.object(p,'run',return_value='a'*40), patch.object(p,'tag_commit',return_value='a'*40):
                with self.assertRaises(ValueError): p.require_publication(self.value)
        with patch.object(p,'run',return_value='b'*40):
            with self.assertRaises(ValueError): p.require_publication(self.value)
        with self.assertRaises(ValueError): p.require_publication('0.1.2')

    def test_notes_must_be_dated_committed_and_derived(self):
        path=Path('docs/releases/v0.2.0.md'); path.parent.mkdir(parents=True)
        path.write_text('# Reader 0.2.0\n\nChange.\n')
        Path('CHANGELOG.md').write_text('## [0.2.0] - 2026-11-22\n\nChange.\n')
        with patch.object(p,'run',return_value=path.read_text().strip()): self.assertEqual(p.release_record(self.value)[0],path)
        for text in ['## [0.2.0]\n\nChange.', '## [0.2.0] - 2026-02-30\n\nChange.', '## [0.2.0] - 2026-11-22\n\nOther.']:
            Path('CHANGELOG.md').write_text(text)
            with self.assertRaises(ValueError): p.release_record(self.value)

    def test_repackage_no_compilation_and_exact_comparison(self):
        candidate=Path(f'target/package/{p.NAME}-{self.value}.crate'); candidate.parent.mkdir(parents=True)
        candidate.write_bytes(b'crate')
        with patch.object(p,'check_rust'), patch.object(p.subprocess,'run') as cargo:
            p.repackage(self.bundle,self.value,'1.99.1')
            self.assertEqual(cargo.call_args.args[0],['cargo','+1.99.1','package','--locked','--no-verify'])
            candidate.write_bytes(b'changed')
            with self.assertRaises(ValueError): p.repackage(self.bundle,self.value,'1.99.1')

    def test_prepare_bootstrap_block_and_signature_barrier(self):
        with patch.object(p,'require_publication'),patch.object(p,'http_get',return_value=None),patch.object(p,'output') as output:
            with self.assertRaisesRegex(ValueError,'bootstrap'): p.prepare(self.bundle,self.value,'1.99.1')
            output.assert_not_called()
        with patch.object(p,'require_publication'),patch.object(p,'http_get',return_value=b'{"crate":{"trustpub_only":true}}'), \
             patch.object(p,'verify_attestations',side_effect=subprocess.CalledProcessError(1,'gh')), \
             patch.object(p,'repackage') as repackage,patch.object(p,'output') as output:
            with self.assertRaises(subprocess.CalledProcessError): p.prepare(self.bundle,self.value,'1.99.1')
            repackage.assert_not_called(); output.assert_not_called()

    def test_existing_registry_is_never_skipped_without_byte_comparison(self):
        with patch.object(p,'registry_metadata',return_value={'checksum':'0'*64}),patch.object(p,'http_get',return_value=b'crate'):
            with self.assertRaises(ValueError): p.registry_compare(self.value,self.bundle/f'{p.NAME}-{self.value}.crate')
        with patch.object(p,'require_publication'),patch.object(p,'http_get',return_value=b'{"crate":{"trustpub_only":true}}'), \
             patch.object(p,'verify_attestations'),patch.object(p,'repackage'),patch.object(p,'registry_metadata',return_value={}), \
             patch.object(p,'registry_compare') as compare,patch.object(p,'output') as output:
            p.prepare(self.bundle,self.value,'1.99.1'); compare.assert_called_once()
            self.assertEqual(output.call_args.kwargs,{'upload':'false'})

    def test_each_payload_signature_binds_reusable_signer_tag_commit(self):
        (self.bundle/f'{p.NAME}-{self.value}.intoto.jsonl').write_text('{}\n')
        with patch.object(p.subprocess,'run') as verify:
            p.verify_attestations(self.bundle,self.value)
        self.assertEqual(verify.call_count,3)
        for call in verify.call_args_list:
            cmd=call.args[0]
            for flag,value in [('--signer-workflow',p.WORKFLOW),('--source-ref','refs/tags/v0.2.0'),('--source-digest','a'*40)]:
                self.assertEqual(cmd[cmd.index(flag)+1],value)

    def test_bad_signature_blocks_all_release_mutations(self):
        with patch.object(p,'require_publication'),patch.object(p,'verify_attestations',side_effect=subprocess.CalledProcessError(1,'gh')), \
             patch.object(p.subprocess,'run') as mutation:
            with self.assertRaises(subprocess.CalledProcessError): p.release_assets(self.bundle,self.value)
            mutation.assert_not_called()

    def release(self, *, draft=True, new=False, missing=False, mismatch=False, wrong_state=False):
        notes=Path('notes.md'); notes.write_text('# Reader 0.2.0\n\nChange.\n')
        names=sorted(p.expected_files(self.value)|{'SHA256SUMS',f'{p.NAME}-{self.value}.intoto.jsonl'})
        (self.bundle/f'{p.NAME}-{self.value}.intoto.jsonl').write_text('{}\n')
        assets=[{'id':i,'name':n,'size':(self.bundle/n).stat().st_size} for i,n in enumerate(names)]
        meta={'id':1,'tag_name':'v0.2.0','name':'Reader 0.2.0','body':notes.read_text(),'draft':draft,'prerelease':False,'assets':[] if missing else assets}
        stages=[dict(meta,assets=assets),dict(meta,assets=assets,draft=False)]
        if wrong_state: stages[0]['draft']=False
        def raw(args,**kwargs):
            i=int(args[2].rsplit('/',1)[-1]);return b'wrong' if mismatch else (self.bundle/names[i]).read_bytes()
        with patch.object(p,'require_publication'),patch.object(p,'verify_attestations'),patch.object(p,'tag_commit',return_value='a'*40), \
             patch.object(p,'release_record',return_value=(notes,'Reader 0.2.0')), \
             patch.object(p,'release_list',side_effect=[[],[meta],[]] if new else [[meta],[]]), \
             patch.object(p,'gh_api',side_effect=stages),patch.object(p.subprocess,'check_output',side_effect=raw), \
             patch.object(p.subprocess,'run') as mutate:
            self.mutate=mutate
            p.release_assets(self.bundle,self.value)
            return mutate.call_args_list

    def test_published_release_never_modified(self):
        self.assertEqual(self.release(draft=False),[])

    def test_mismatch_published_asset_blocks_mutation(self):
        with self.assertRaises(ValueError): self.release(draft=False,mismatch=True)
        self.mutate.assert_not_called()

    def test_draft_checked_assets_verified_before_and_after_publish(self):
        calls=self.release(missing=True)
        self.assertEqual(sum('POST' in c.args[0] for c in calls),5)
        self.assertIn('PATCH',calls[-1].args[0])
        self.assertTrue(all('--clobber' not in c.args[0] for c in calls))

    def test_unexpected_draft_state_blocks_publication(self):
        with self.assertRaises(ValueError): self.release(wrong_state=True)
        self.mutate.assert_not_called()

    def test_new_release_creates_verified_draft(self):
        calls=self.release(new=True,draft=True)
        command=calls[0].args[0]
        for flag in ['--verify-tag','--draft','--latest=false','--notes-file']: self.assertIn(flag,command)

    def test_exact_release_body_and_state(self):
        meta={'tag_name':'v0.2.0','name':'Reader','body':'# Reader\n','draft':True,'prerelease':False}
        p.check_release_metadata(meta,self.value,'Reader','# Reader\n',True)
        for key,value in [('body','# Reader\n\n'),('tag_name','v0.3.0'),('draft',False),('prerelease',True)]:
            with self.assertRaises(ValueError): p.check_release_metadata(dict(meta,**{key:value}),self.value,'Reader','# Reader\n',True)

    def test_latest_semver_not_time_and_no_draft_or_prerelease(self):
        self.assertEqual(p.latest_policy('1.9.0',[{'tag_name':'v1.10.0','draft':False,'prerelease':False}]),(False,False))
        self.assertEqual(p.latest_policy('1.9.0',[{'tag_name':'v9.0.0','draft':True,'prerelease':False}]),(False,True))
        self.assertEqual(p.latest_policy('9.0.0-rc.1',[]),(True,False))

    def test_docsrs_pending_fails_at_deadline(self):
        # Freeze comparison and signatures are checked separately above.
        with patch.object(p,'require_publication'),patch.object(p,'download_release'),patch.object(p,'expected_files',return_value=set()), \
             patch.object(p,'verify_attestations'),patch.object(p,'check_archive'),patch.object(p,'registry_compare'), \
             patch.object(p.Path,'read_bytes',return_value=b'same'),patch.object(p,'http_get',return_value=b'{"version":"0.2.0","doc_status":false}'), \
             patch.object(p.time,'sleep'),patch.dict(os.environ,{'RUNNER_TEMP':str(self.root)}):
            with self.assertRaisesRegex(ValueError,'pending is not passed'): p.verify_publication(self.bundle,self.value)

    def test_historical_layouts_exact_digest_pins(self):
        records=p.legacy_records()
        self.assertEqual(set(records),{'0.1.0','0.1.1','0.1.2'})
        for value,files in records.items():
            self.assertEqual(set(files),{p.archive_name(value),p.archive_name(value)+'.sha256'})
        with patch.object(p,'gh_api',return_value={'draft':False,'assets':[]}),patch.object(p,'http_get') as download:
            with self.assertRaises(ValueError): p.verify_legacy('0.1.2')
            download.assert_not_called()

    def test_known_good_fixture_independent_from_live_download(self):
        fixture=Path(p.__file__).parent/'fixtures/irig106-types-0.1.0.crate'
        self.assertEqual(p.digest(fixture),p.KNOWN_SHA256)
        with patch.object(p,'KNOWN_SHA256','0'*64),patch.object(p,'http_get') as download:
            with self.assertRaises(ValueError): p.known_good()
            download.assert_not_called()

    def test_workflow_permission_boundary_and_immutable_output(self):
        core=(REPO_ROOT/'.github/workflows/build-binaries-core.yml').read_text()
        wrapper=(REPO_ROOT/'.github/workflows/build-binaries.yml').read_text()
        caller=(REPO_ROOT/'.github/workflows/trusted-publishing.yml').read_text()
        self.assertNotIn('id-token:',core); self.assertNotIn('attestations:',core)
        dry=caller.split('  build-dry-run:\n')[1].split('  publish-path-check:\n')[0]
        self.assertNotIn('id-token:',dry); self.assertIn('build-binaries-core.yml',dry)
        for job in ['build','publish','github-release','verify','verify-windows']:
            block=caller.split('  '+job+':\n')[1]
            header=block.split('    steps:')[0].split('    uses:')[0]
            for guard in ["github.event_name == 'push'","github.ref_type == 'tag'","startsWith(github.ref, 'refs/tags/v')"]:
                self.assertIn(guard,header)
        self.assertIn("inputs.publish && github.event_name == 'push'",wrapper)
        self.assertIn('name: Checksums and attestations',wrapper)
        for text in [core,wrapper,caller]:
            self.assertNotIn('overwrite:',text)
            self.assertEqual(text.count('uses: actions/checkout@'),text.count('ref: ${{ github.sha }}'))
            self.assertNotIn('--clobber',text)


if __name__=='__main__': unittest.main()
