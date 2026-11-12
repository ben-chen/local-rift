"""External JSON protocol acceptance, with authored exact-40-card substitutions."""
import hashlib,json,subprocess,sys,time
from pathlib import Path
binary=Path(sys.argv[1]).resolve()
output=Path(sys.argv[2]).resolve()
rows=[]
coverage={str(card):{level:[False,False] for level in ['easy','tactical','strategic']} for card in [441,442]}
process=subprocess.Popen([str(binary)],stdin=subprocess.PIPE,stdout=subprocess.PIPE,text=True,bufsize=1)
revision=0
trace=hashlib.sha256()
counts={}
def request(command,**fields):
 global revision
 payload={'command':command,**fields}
 if command not in ['catalog','observe','info','validate_deck']:payload['revision']=revision
 process.stdin.write(json.dumps(payload,separators=(',',':'))+'\n');process.stdin.flush()
 line=process.stdout.readline()
 assert line,'API ended without a response'
 response=json.loads(line)
 trace.update(json.dumps([payload,response],sort_keys=True,separators=(',',':')).encode())
 counts[command]=counts.get(command,0)+1
 assert response['ok'],(payload,response)
 revision=response['revision']
 return response['result']
try:
 catalog=request('catalog')
 deck=catalog['legends'][catalog['presets'][1]['legend_id']]['training_deck']
 deck['name']='Master Yi / reaction package API verification'
 deck['cards']=[[{6:441,7:442}.get(card,card),count] for card,count in deck['cards']]
 assert request('validate_deck',deck=deck)['card_count']==40
 assert catalog['cards'][442]['play_targets']['ordered_slots'][1]['includes_repeat_choices']
 for level,difficulty in enumerate(['easy','tactical','strategic']):
  for opponent in [0,5,3]:
   for seat in range(2):
    for starter in range(2):
     trace=hashlib.sha256();counts={};seen=set();previews=0;started=time.monotonic()
     ids=[1,opponent];custom=[deck,None]
     if seat:ids.reverse();custom.reverse()
     seed=442000+level*100+opponent*10+seat*2+starter
     request('reset',decks=ids,custom_decks=custom,seed=seed,difficulty=difficulty,first_player=starter)
     request('start')
     for step in range(2400):
      public=request('observe',player=seat)
      if public['players'][1-seat]['hand'] is not None:
       decision=public['decision']
       assert decision and decision['type'] in ['reveal_hand','sabotage','recycle_revealed_hand'] and decision['opponent']==1-seat
      for item in public['chain']:
       if item['card_id'] in [441,442] and item['ability'] is None:
        seen.add(item['card_id']);coverage[str(item['card_id'])][difficulty][item['player']]=True
      if public['winner'] is not None:break
      actor=public['priority']
      if actor==seat and public['decision'] is None and public['chain'] and 442 in public['players'][seat]['hand']:
       action={'type':'play_card_targets','hand_index':public['players'][seat]['hand'].index(442),'lane':None,'targets':[]}
       current=revision
       first=request('target_candidates',player=seat,execution=0,action=action)
       assert first['next_slot']['kind']=='unit'
       for unit in first['candidates']:
        action['targets']=[unit]
        second=request('target_candidates',player=seat,execution=0,action=action)
        assert second['next_slot']['kind']=='spell' and second['candidates']
        for spell in second['candidates']:
         action['targets']=[unit,spell]
         final=request('target_candidates',player=seat,execution=0,action=action)
         assert final['next_slot'] is None and final['targets']==[unit,spell]
         previews+=1
       assert revision==current
       assert request('observe',player=seat)==public
      request('step_ai',player=actor)
     else:raise AssertionError((difficulty,opponent,seat,starter,'game did not finish'))
     row={'difficulty':difficulty,'opponent':opponent,'seat':seat,'starter':starter,'seed':seed,'actions':step,'winner':public['winner'],'scores':[p['score'] for p in public['players']],'cards_played':sorted(seen),'linked_target_previews':previews,'requests':counts,'trace_sha256':trace.hexdigest(),'seconds':round(time.monotonic()-started,2)}
     rows.append(row)
     result={'binary':str(binary),'binary_sha256':hashlib.sha256(binary.read_bytes()).hexdigest(),'runner_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'games':rows,'coverage':coverage,'status':'running'}
     output.write_text(json.dumps(result,indent=2)+'\n')
     print(difficulty,opponent,seat,starter,'steps',step,'previews',previews,'played',sorted(seen),flush=True)
 assert sum(row['linked_target_previews'] for row in rows)>0
 assert all(any(row['linked_target_previews'] for row in rows if row['seat']==seat) for seat in range(2))
 result['status']='passed'
 result['scope']='36 complete games, all levels/opponents/seats/starters; observed card use recorded separately. Pure ordered target previews exercised on actual dealt hands.'
 output.write_text(json.dumps(result,indent=2)+'\n')
 print('PASS',len(rows),'games',sum(row['linked_target_previews'] for row in rows),'linked target previews',coverage,flush=True)
finally:
 process.stdin.close();process.wait(timeout=5);process.stdout.close()
