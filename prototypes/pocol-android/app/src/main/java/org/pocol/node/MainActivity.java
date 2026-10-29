package org.pocol.node;

import android.Manifest;
import android.app.*;
import android.content.*;
import android.content.pm.PackageManager;
import android.graphics.Color;
import android.graphics.Typeface;
import android.graphics.drawable.GradientDrawable;
import android.os.*;
import android.text.InputType;
import android.view.*;
import android.widget.*;
import java.io.*;
import java.nio.charset.StandardCharsets;
import org.json.*;

/** Native wallet. Only public data crosses the service bridge; signing remains in Rust. */
public final class MainActivity extends Activity {
    private static final int BLUE=Color.rgb(26,169,228), INK=Color.rgb(17,38,57), MUTED=Color.rgb(100,119,138), BG=Color.rgb(245,248,252);
    private final Handler main=new Handler(Looper.getMainLooper());
    private Messenger service;
    private boolean bound, updating, requestPending;
    private String home,address="",tab="Wallet";
    private long responseId;
    private JSONObject snapshot=new JSONObject();
    private LinearLayout content,nav;
    private TextView balance,connectionStatus,miningStatus,headerStatus;
    private Switch miningSwitch;
    private AlertDialog quoteDialog;
    private final Messenger receiver=new Messenger(new Handler(Looper.getMainLooper(),msg->{
        if(msg.what==1) update(msg.getData().getString("status","{}"));
        if(msg.what==3) try {JSONObject r=new JSONObject(msg.getData().getString("result","{}"));if(r.has("error")){requestPending=false;notice(r.optString("error"));}}catch(JSONException ignored){}
        return true;
    }));
    private final ServiceConnection connection=new ServiceConnection(){
        public void onServiceConnected(ComponentName name,IBinder binder){service=new Messenger(binder);responseId=0;}
        public void onServiceDisconnected(ComponentName name){service=null;requestPending=false;render();}
    };
    private final Runnable refresh=new Runnable(){public void run(){
        if(service!=null){Message request=Message.obtain(null,1);request.replyTo=receiver;try{service.send(request);}catch(RemoteException e){service=null;}}
        else {File result=new File(getFilesDir(),"last-result.json");if(result.isFile())try(InputStream in=new FileInputStream(result)){JSONObject r=new JSONObject(new String(readLimited(in),StandardCharsets.UTF_8));if(r.has("error"))headerStatus.setText(r.optString("error"));}catch(Exception ignored){}}
        main.postDelayed(this,2000);
    }};
    private int dp(int n){return (int)(n*getResources().getDisplayMetrics().density+.5f);}
    private GradientDrawable shape(int color,int radius){GradientDrawable d=new GradientDrawable();d.setColor(color);d.setCornerRadius(dp(radius));return d;}
    private TextView text(String value,int size,int color,boolean bold){TextView t=new TextView(this);t.setText(value);t.setTextSize(size);t.setTextColor(color);if(bold)t.setTypeface(Typeface.create("sans-serif-medium",Typeface.NORMAL));t.setPadding(0,dp(4),0,dp(4));return t;}
    private LinearLayout column(){LinearLayout l=new LinearLayout(this);l.setOrientation(LinearLayout.VERTICAL);return l;}
    private LinearLayout card(LinearLayout parent){LinearLayout c=column();c.setPadding(dp(20),dp(18),dp(20),dp(18));c.setBackground(shape(Color.WHITE,24));LinearLayout.LayoutParams p=new LinearLayout.LayoutParams(-1,-2);p.setMargins(0,0,0,dp(16));parent.addView(c,p);return c;}
    private void gap(LinearLayout p,int n){Space s=new Space(this);p.addView(s,new LinearLayout.LayoutParams(1,dp(n)));}
    private Button button(LinearLayout p,String label,boolean primary,Runnable action){Button b=new Button(this);b.setText(label);b.setAllCaps(false);b.setTextSize(15);b.setTextColor(primary?INK:Color.rgb(8,119,159));b.setTypeface(Typeface.create("sans-serif-medium",0));b.setBackground(shape(primary?BLUE:Color.rgb(232,247,253),16));b.setMinHeight(dp(52));LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(-1,dp(54));lp.setMargins(0,dp(8),0,0);p.addView(b,lp);b.setOnClickListener(v->action.run());return b;}
    @Override public void onCreate(Bundle state){super.onCreate(state);home=new File(getFilesDir(),"pocol").getAbsolutePath();getWindow().setStatusBarColor(BG);getWindow().setNavigationBarColor(Color.WHITE);getWindow().getDecorView().setSystemUiVisibility(View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR|View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR);
        address=getPreferences(MODE_PRIVATE).getString("address","");
        if(address.isEmpty())try{JSONObject r=new JSONObject(Native.identity(home));address=r.optString("address");if(!address.isEmpty())getPreferences(MODE_PRIVATE).edit().putString("address",address).apply();else notice(r.optString("error","Cannot create identity"));}catch(JSONException e){notice("Cannot create identity");}
        render();
    }
    private void render(){
        balance=null;connectionStatus=null;miningSwitch=null;miningStatus=null;
        LinearLayout root=column();root.setBackgroundColor(BG);root.setPadding(dp(22),dp(12),dp(22),0);
        root.setOnApplyWindowInsetsListener((v,i)->{v.setPadding(dp(22),dp(12)+i.getSystemWindowInsetTop(),dp(22),i.getSystemWindowInsetBottom());return i;});setContentView(root);
        LinearLayout top=new LinearLayout(this);top.setGravity(Gravity.CENTER_VERTICAL);root.addView(top);TextView brand=text("◈  PoCol",25,INK,true);top.addView(brand,new LinearLayout.LayoutParams(0,-2,1));TextView badge=text(" TEST NETWORK ",11,Color.rgb(8,119,159),true);badge.setBackground(shape(Color.rgb(226,245,253),12));badge.setPadding(dp(10),dp(7),dp(10),dp(7));top.addView(badge);
        headerStatus=text(service==null?"Wallet offline":"Connecting to peers…",12,MUTED,false);root.addView(headerStatus);gap(root,12);
        ScrollView scroll=new ScrollView(this);scroll.setFillViewport(true);scroll.setClipToPadding(false);root.addView(scroll,new LinearLayout.LayoutParams(-1,0,1));content=column();scroll.addView(content);
        if(tab.equals("Wallet")) wallet();else if(tab.equals("Mining")) mining();else if(tab.equals("Activity")) activity();else settings();
        nav=new LinearLayout(this);nav.setPadding(0,dp(8),0,dp(8));root.addView(nav);
        for(String name:new String[]{"Wallet","Activity","Mining","Settings"}){TextView item=text(name,12,name.equals(tab)?Color.rgb(8,119,159):MUTED,name.equals(tab));item.setGravity(Gravity.CENTER);item.setMinHeight(dp(52));if(name.equals(tab))item.setBackground(shape(Color.rgb(227,245,253),16));nav.addView(item,new LinearLayout.LayoutParams(0,-2,1));item.setOnClickListener(v->{tab=name;render();});}
        showSnapshot();
    }
    private void wallet(){
        content.addView(text("Your wallet. Your network.",23,INK,true));gap(content,14);
        LinearLayout hero=card(content);GradientDrawable bg=new GradientDrawable(GradientDrawable.Orientation.TL_BR,new int[]{INK,Color.rgb(13,65,94)});bg.setCornerRadius(dp(26));hero.setBackground(bg);
        hero.addView(text("AVAILABLE BALANCE",11,Color.rgb(167,201,220),true));balance=text("—",38,Color.WHITE,true);hero.addView(balance);hero.addView(text("PCOL · Test currency",14,Color.rgb(127,219,255),false));gap(hero,20);TextView account=text(shortAddress(),12,Color.rgb(191,218,233),false);hero.addView(account);account.setOnClickListener(v->receive());
        LinearLayout actions=new LinearLayout(this);content.addView(actions);for(String label:new String[]{"↗  Send","↙  Receive"}){LinearLayout c=column();LinearLayout.LayoutParams lp=new LinearLayout.LayoutParams(0,-2,1);lp.setMargins(label.contains("Send")?0:dp(6),0,label.contains("Send")?dp(6):0,dp(18));actions.addView(c,lp);button(c,label,true,()->{if(label.contains("Send"))send();else receive();});}
        LinearLayout asset=card(content);asset.addView(text("YOUR ASSET",11,MUTED,true));asset.addView(text("◈  PoCol",20,INK,true));asset.addView(text("Native asset · PCOL",13,MUTED,false));asset.addView(text("Test coins have no market price.",12,MUTED,false));
        LinearLayout status=card(content);status.addView(text("Connected, on your terms",18,INK,true));connectionStatus=text("Start your local node to see verified balances.",14,MUTED,false);status.addView(connectionStatus);button(status,service==null?"Connect wallet":"Manage mining",false,()->{if(service==null)startNode();else{tab="Mining";render();}});
    }
    private void mining(){content.addView(text("Mining",30,INK,true));content.addView(text("Contribute when it works for you.",15,MUTED,false));gap(content,18);LinearLayout c=card(content);c.addView(text("Device mining",20,INK,true));miningSwitch=new Switch(this);miningSwitch.setText("Enable mining");miningSwitch.setTextSize(16);miningSwitch.setTextColor(INK);miningSwitch.setPadding(0,dp(16),0,dp(16));c.addView(miningSwitch);miningStatus=text("Connect your wallet first.",14,MUTED,false);c.addView(miningStatus);
        miningSwitch.setOnCheckedChangeListener((b,on)->{if(!updating){updating=true;b.setChecked(snapshot.optBoolean("miningEnabled"));updating=false;command(object("method","mining","enabled",on));}});
        button(c,"Review mining registration",false,()->command(object("method","quote_register")));
        LinearLayout note=card(content);note.addView(text("You stay in control",18,INK,true));note.addView(text("Turning mining off keeps your wallet connected and validating. Turning it on requires an eligible registered account. Registration locks a bond and pays a network fee.",14,MUTED,false));note.addView(text("Mining uses battery, mobile data and storage. Android may pause the app. Rewards depend on accepted work and are not guaranteed.",14,MUTED,false));}
    private void activity(){content.addView(text("Activity",30,INK,true));content.addView(text("Transactions submitted from this device",13,MUTED,false));gap(content,18);JSONArray items=snapshot.optJSONArray("activity");if(items==null||items.length()==0){LinearLayout c=card(content);c.addView(text("A fresh start",22,INK,true));c.addView(text("Your transfers and mining registrations will appear here after you submit them.",15,MUTED,false));return;}
        for(int i=0;i<items.length();i++){JSONObject a=items.optJSONObject(i);if(a==null)continue;LinearLayout c=card(content);c.addView(text(a.optString("kind").equals("registration")?"Mining registration":"Sent PCOL",17,INK,true));c.addView(text(a.optString("amount")+" PCOL",22,INK,true));c.addView(text(a.optString("state")+(a.has("confirmations")?" · "+a.optLong("confirmations")+" confirmations":""),13,MUTED,false));TextView hash=text(a.optString("hash"),11,MUTED,false);hash.setTextIsSelectable(true);c.addView(hash);}
    }
    private void settings(){content.addView(text("Settings",30,INK,true));gap(content,18);LinearLayout c=card(content);c.addView(text("Network",20,INK,true));c.addView(text("Import the shared network file once. Your private key stays on this device.",14,MUTED,false));button(c,"Import network file",true,()->{if(service!=null){notice("Disconnect the wallet before importing a network.");return;}startActivityForResult(new Intent(Intent.ACTION_OPEN_DOCUMENT).setType("application/json").addCategory(Intent.CATEGORY_OPENABLE),1);});button(c,"Connect wallet",false,this::startNode);button(c,"Disconnect wallet",false,()->{if(service!=null)try{service.send(Message.obtain(null,2));headerStatus.setText("Disconnecting…");}catch(RemoteException ignored){}});
        LinearLayout identity=card(content);identity.addView(text("Your account",20,INK,true));TextView a=text(address,12,MUTED,false);a.setTextIsSelectable(true);identity.addView(a);button(identity,"Copy public address",false,this::copy);identity.addView(text("This test build stores its key on this phone. Uninstalling erases the account and local state. Portable recovery export is not available in this build.",13,MUTED,false));
    }
    private void send(){if(service==null){notice("Connect your wallet before sending.");return;}LinearLayout form=column();form.setPadding(dp(24),dp(12),dp(24),dp(12));form.addView(text("Recipient address",13,MUTED,true));EditText to=new EditText(this);to.setHint("0x…");to.setSingleLine(true);to.setInputType(InputType.TYPE_CLASS_TEXT|InputType.TYPE_TEXT_FLAG_NO_SUGGESTIONS);form.addView(to);form.addView(text("Amount in PCOL",13,MUTED,true));EditText amount=new EditText(this);amount.setHint("0.00");amount.setInputType(InputType.TYPE_CLASS_NUMBER|InputType.TYPE_NUMBER_FLAG_DECIMAL);form.addView(amount);form.addView(text("You will review the recipient, amount and maximum fee before signing.",13,MUTED,false));
        AlertDialog dialog=new AlertDialog.Builder(this).setTitle("Send PCOL").setView(form).setNegativeButton("Cancel",null).setPositiveButton("Review",null).create();dialog.setOnShowListener(v->dialog.getButton(-1).setOnClickListener(b->{if(requestPending)return;command(object("method","quote_send","to",to.getText().toString().trim(),"amount",amount.getText().toString().trim()));dialog.dismiss();}));dialog.show();}
    private void receive(){LinearLayout c=column();c.setPadding(dp(24),dp(12),dp(24),dp(18));c.addView(text("Receive PCOL",24,INK,true));c.addView(text("Use this address on the same PoCol network only.",14,MUTED,false));TextView a=text(address,17,INK,true);a.setTextIsSelectable(true);c.addView(a);button(c,"Copy address",true,this::copy);button(c,"Share address",false,()->startActivity(Intent.createChooser(new Intent(Intent.ACTION_SEND).setType("text/plain").putExtra(Intent.EXTRA_TEXT,address),"Share PoCol address")));new AlertDialog.Builder(this).setView(c).setNegativeButton("Done",null).show();}
    private void review(JSONObject q){requestPending=false;String id=q.optString("quoteId");boolean registration=q.optString("kind").equals("registration");String details="Network: "+q.optLong("chainId")+"\n\nTo\n"+q.optString("to")+"\n\n"+(registration?"Bond":"Amount")+": "+q.optString("amount")+" PCOL\nMaximum fee: "+q.optString("maximumFee")+" PCOL\nMaximum total: "+q.optString("maximumTotal")+" PCOL\n\nReview expires in 60 seconds.";
        quoteDialog=new AlertDialog.Builder(this).setTitle(registration?"Review registration":"Review transfer").setMessage(details).setNegativeButton("Cancel",(d,w)->command(object("method","cancel"))).setPositiveButton("Confirm & sign",(d,w)->command(object("method","confirm","quote_id",id))).setOnCancelListener(d->command(object("method","cancel"))).create();quoteDialog.show();}
    private void update(String raw){try{JSONObject v=new JSONObject(raw);if(v.has("address")){snapshot=v;showSnapshot();long id=v.optLong("responseId");if(id>responseId){responseId=id;JSONObject r=v.optJSONObject("response");if(r!=null){requestPending=false;String type=r.optString("type");if(type.equals("quote"))review(r);else if(type.equals("error"))notice(r.optString("message"));else if(type.equals("submitted"))new AlertDialog.Builder(this).setTitle("Transaction submitted").setMessage("Waiting for network inclusion.\n\n"+r.optString("hash")).setPositiveButton("Done",null).show();}}}else if(v.has("error")){headerStatus.setText(v.optString("error"));}else headerStatus.setText("Preparing local validation…");}catch(JSONException ignored){}}
    private void showSnapshot(){if(headerStatus==null)return;if(service==null){headerStatus.setText("Wallet offline · balances unavailable");return;}if(!snapshot.has("balance"))return;if(tab.equals("Activity")){content.removeAllViews();activity();}headerStatus.setText(snapshot.optInt("peers")+" peers · Block "+snapshot.optLong("height"));if(balance!=null)balance.setText(snapshot.optString("balance","—"));if(connectionStatus!=null)connectionStatus.setText(snapshot.optInt("peers")>0?"Your device is validating blocks. Mining is "+(snapshot.optBoolean("miningEnabled")?"on.":"off."):"Looking for peers. Your balance may be out of date.");if(miningSwitch!=null){updating=true;miningSwitch.setChecked(snapshot.optBoolean("miningEnabled"));miningSwitch.setEnabled(snapshot.optBoolean("miningEligible")||snapshot.optBoolean("miningEnabled"));updating=false;}if(miningStatus!=null)miningStatus.setText(snapshot.optString("miningState")+"\nAccepted blocks this session: "+snapshot.optLong("blocksMined")+"\nRegistration bond: "+snapshot.optString("bond")+" PCOL");}
    private void startNode(){if(!new File(home,"network.pocol.json").isFile()){notice("Import the shared network file in Settings first.");tab="Settings";render();return;}if(Build.VERSION.SDK_INT>=33&&checkSelfPermission(Manifest.permission.POST_NOTIFICATIONS)!=PackageManager.PERMISSION_GRANTED)requestPermissions(new String[]{Manifest.permission.POST_NOTIFICATIONS},2);try{startForegroundService(new Intent(this,NodeService.class));attach();headerStatus.setText("Starting local validation…");}catch(RuntimeException e){notice("Cannot start: "+e.getMessage());}}
    private void command(JSONObject command){if(service==null){notice("Connect your wallet first.");return;}if(requestPending){notice("Please wait for the previous request.");return;}Message m=Message.obtain(null,3);Bundle b=new Bundle();b.putString("command",command.toString());m.setData(b);m.replyTo=receiver;try{service.send(m);requestPending=true;main.postDelayed(()->requestPending=false,10000);}catch(RemoteException e){notice("Wallet service unavailable");}}
    private static JSONObject object(Object... pairs){JSONObject j=new JSONObject();try{for(int i=0;i<pairs.length;i+=2)j.put((String)pairs[i],pairs[i+1]);}catch(JSONException ignored){}return j;}
    private String shortAddress(){return address.length()==42?address.substring(0,8)+"…"+address.substring(36):"Account unavailable";}
    private void copy(){getSystemService(android.content.ClipboardManager.class).setPrimaryClip(ClipData.newPlainText("PoCol address",address));notice("Address copied");}
    private void notice(String message){Toast.makeText(this,message,Toast.LENGTH_LONG).show();}
    private void attach(){if(!bound)bound=bindService(new Intent(this,NodeService.class),connection,0);}
    @Override protected void onStart(){super.onStart();attach();main.post(refresh);}
    @Override protected void onStop(){main.removeCallbacks(refresh);if(bound){unbindService(connection);bound=false;service=null;}super.onStop();}
    private static byte[] readLimited(InputStream in)throws IOException{ByteArrayOutputStream out=new ByteArrayOutputStream();byte[] b=new byte[8192];int n;while((n=in.read(b))!=-1){if(out.size()+n>1048576)throw new IOException("File exceeds 1 MiB");out.write(b,0,n);}return out.toByteArray();}
    @Override protected void onActivityResult(int request,int result,Intent data){super.onActivityResult(request,result,data);if(request!=1||result!=RESULT_OK||data==null||data.getData()==null)return;new Thread(()->{File incoming=new File(getCacheDir(),"network-import.json");try(InputStream in=getContentResolver().openInputStream(data.getData())){if(in==null)throw new IOException("Cannot read file");try(FileOutputStream out=new FileOutputStream(incoming)){out.write(readLimited(in));}JSONObject r=new JSONObject(Native.importNetwork(home,incoming.getAbsolutePath()));main.post(()->{if(r.has("error"))notice(r.optString("error"));else{notice("Network imported");startNode();}});}catch(Exception e){main.post(()->notice("Import failed: "+e.getMessage()));}finally{incoming.delete();}},"network-import").start();}
}
